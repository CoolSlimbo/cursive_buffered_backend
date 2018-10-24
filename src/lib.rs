extern crate crossbeam_channel;
extern crate cursive;
extern crate enumset;
extern crate smallvec;
extern crate unicode_segmentation;
extern crate unicode_width;

use crossbeam_channel::{Receiver, Sender};
use cursive::backend::{Backend, InputRequest};
use cursive::event::Event;
use cursive::theme;
use cursive::Vec2;
use enumset::EnumSet;
use std::cell::RefCell;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod smallstring;

use smallstring::SmallString;

pub struct BufferedBackend {
    backend: Box<Backend>,
    buf: RefCell<Vec<Option<(Style, SmallString)>>>,
    w: usize,
    h: usize,
    current_style: RefCell<Style>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Style {
    effects: EnumSet<theme::Effect>,
    color_pair: theme::ColorPair,
}

fn background_style() -> Style {
    Style {
        effects: EnumSet::new(),
        color_pair: theme::ColorPair {
            front: theme::Color::Dark(theme::BaseColor::Black),
            back: theme::Color::Dark(theme::BaseColor::Black),
        },
    }
}

fn write_style(backend: &Backend, style: &Style) {}

impl BufferedBackend {
    pub fn new(backend: Box<Backend>) -> Self {
        let screen_size = backend.screen_size();
        let w = screen_size.x;
        let h = screen_size.y;
        let style = background_style();
        let buf = std::iter::repeat(Some((style, " ".into())))
            .take(w as usize * h as usize)
            .collect();
        BufferedBackend {
            backend,
            buf: RefCell::new(buf),
            w: w as usize,
            h: h as usize,
            current_style: RefCell::new(style),
        }
    }

    fn clear(&self, new_style: Style) {
        for cell in self.buf.borrow_mut().iter_mut() {
            match *cell {
                Some((ref mut style, ref mut text)) => {
                    *style = new_style;
                    text.clear();
                    text.push_str(" ");
                }
                _ => {
                    *cell = Some((new_style, " ".into()));
                }
            }
        }
    }

    fn resize(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        self.buf
            .borrow_mut()
            .resize(w * h, Some((background_style(), " ".into())));
    }

    fn present(&mut self) {
        let buf = self.buf.borrow();

        let mut last_style = background_style();
        write_style(&*self.backend, &last_style);

        let mut pos = Vec2::new(0, 0);
        while pos.y < self.h {
            pos.x = 0;
            while pos.x < self.w {
                if let Some((style, ref text)) = buf[pos.y * self.w + pos.x] {
                    if style != last_style {
                        write_style(&*self.backend, &style);
                        last_style = style;
                    }
                    self.backend.print_at(pos, text);
                }
                pos.x += 1;
            }
            pos.y += 1;
        }

        // Make sure everything is written out
        self.backend.refresh();
    }

    fn draw(&self, x: usize, y: usize, text: &str, style: Style) {
        if y < self.h {
            let mut buf = self.buf.borrow_mut();
            let mut x = x;
            for g in UnicodeSegmentation::graphemes(text, true) {
                let width = UnicodeWidthStr::width(g);
                if width > 0 {
                    if x < self.w {
                        buf[y * self.w + x] = Some((style, g.into()));
                    }
                    x += 1;
                    for _ in 0..(width - 1) {
                        if x < self.w {
                            buf[y * self.w + x] = None;
                        }
                        x += 1;
                    }
                }
            }
        }
    }
}

impl Backend for BufferedBackend {
    // TODO: take `self` by value?
    // Or implement Drop?
    /// Prepares to close the backend.
    ///
    /// This should clear any state in the terminal.
    fn finish(&mut self) {
        self.backend.finish();
    }

    /// Starts a thread to collect input and send it to the given channel.
    ///
    /// `event_trigger` will receive a value before any event is needed.
    fn start_input_thread(
        &mut self,
        event_sink: Sender<Option<Event>>,
        input_request: Receiver<InputRequest>,
    ) {
        self.backend.start_input_thread(event_sink, input_request);
    }

    /// Prepares the backend to collect input.
    ///
    /// This is only required for non-thread-safe backends like BearLibTerminal
    /// where we cannot collect input in a separate thread.
    fn prepare_input(&mut self, input_request: InputRequest) {
        self.backend.prepare_input(input_request);
    }

    /// Refresh the screen.
    fn refresh(&mut self) {
        //TODO
    }

    /// Should return `true` if this backend supports colors.
    fn has_colors(&self) -> bool {
        self.backend.has_colors()
    }

    /// Returns the screen size.
    fn screen_size(&self) -> Vec2 {
        self.backend.screen_size()
    }

    /// Main method used for printing
    fn print_at(&self, pos: Vec2, text: &str) {
        //TODO
    }

    /// Clears the screen with the given color.
    fn clear(&self, color: theme::Color) {
        //TODO
    }

    /// Starts using a new color.
    ///
    /// This should return the previously active color.
    fn set_color(&self, colors: theme::ColorPair) -> theme::ColorPair {
        let mut current_style = self.current_style.borrow_mut();
        let previous_colors = current_style.color_pair;
        current_style.color_pair = colors;
        previous_colors
    }

    /// Enables the given effect.
    fn set_effect(&self, effect: theme::Effect) {
        let mut current_style = self.current_style.borrow_mut();
        current_style.effects.insert(effect);
    }

    /// Disables the given effect.
    fn unset_effect(&self, effect: theme::Effect) {
        let mut current_style = self.current_style.borrow_mut();
        current_style.effects.remove(effect);
    }
}
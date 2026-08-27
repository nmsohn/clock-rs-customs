pub mod counter;
pub mod mode;
pub mod time_zone;

use std::{
    io::{BufWriter, StdoutLock, Write},
    time::Duration,
};

use crate::{
    character::Character, clock::mode::ClockMode, color::Color, config::Config, error::Error,
    position::Position,
};

#[derive(Default)]
pub struct Padding {
    pub top: u16,
    clock: String,
    text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LayoutKind {
    Ascii,
    Compact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Layout {
    kind: LayoutKind,
    show_seconds: bool,
    show_footer: bool,
}

impl Layout {
    const fn ascii(show_seconds: bool, show_footer: bool) -> Self {
        Self {
            kind: LayoutKind::Ascii,
            show_seconds,
            show_footer,
        }
    }

    const fn compact(show_seconds: bool) -> Self {
        Self {
            kind: LayoutKind::Compact,
            show_seconds,
            show_footer: false,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::ascii(true, true)
    }
}

pub struct Clock {
    pub mode: ClockMode,
    pub padding: Padding,
    pub interval: Duration,
    pub x_pos: Position,
    pub y_pos: Position,
    pub color: Color,
    pub use_12h: bool,
    pub hide_seconds: bool,
    pub blink: bool,
    pub bold: bool,
    layout: Layout,
}

impl Clock {
    const DIGIT_WIDTH: u16 = 7;
    const COLON_WIDTH: u16 = 5;
    const WIDTH_NO_SECONDS: u16 = Self::DIGIT_WIDTH * 4 + Self::COLON_WIDTH;
    const WIDTH: u16 = Self::WIDTH_NO_SECONDS + Self::COLON_WIDTH + Self::DIGIT_WIDTH * 2;
    const ASCII_HEIGHT: u16 = 5;
    const FOOTER_HEIGHT: u16 = 2;
    const AM_SUFFIX: &'static str = " [AM]";
    const PM_SUFFIX: &'static str = " [PM]";
    const COMPACT_AM_SUFFIX: &'static str = " AM";
    const COMPACT_PM_SUFFIX: &'static str = " PM";

    pub fn new(config: Config, mode: ClockMode) -> Self {
        let show_seconds = !config.date.hide_seconds;

        Self {
            mode,
            padding: Padding::default(),
            interval: Duration::from_millis(config.general.interval),
            x_pos: config.position.x,
            y_pos: config.position.y,
            color: config.general.color,
            use_12h: config.date.use_12h,
            hide_seconds: config.date.hide_seconds,
            blink: config.general.blink,
            bold: config.general.bold,
            layout: Layout::ascii(show_seconds, true),
        }
    }

    pub fn update_padding(&mut self, width: u16, height: u16) -> Result<(), Error> {
        let layout = self.layout(width, height);
        let layout_width = self.layout_width(layout);
        let half_width = layout_width / 2;

        let column = self.x_pos.calculate(width, half_width);
        self.padding.top = self.y_pos.calculate(height, self.layout_height(layout) / 2);

        self.padding.clock = " ".repeat(column as usize);
        self.padding.text.clear();

        if layout.kind == LayoutKind::Ascii && layout.show_footer {
            let text_len = self.footer_text(layout_width)?.len() as u16;
            self.padding.text = format!(
                "{}{}",
                self.padding.clock,
                " ".repeat(half_width.saturating_sub(text_len / 2) as usize)
            );
        }

        self.layout = layout;

        Ok(())
    }

    fn layout(&self, width: u16, height: u16) -> Layout {
        let show_seconds = !self.hide_seconds;

        for layout in [
            Layout::ascii(show_seconds, true),
            Layout::ascii(false, true),
            Layout::ascii(show_seconds, false),
            Layout::ascii(false, false),
            Layout::compact(show_seconds),
            Layout::compact(false),
        ] {
            if self.layout_fits(layout, width, height) {
                return layout;
            }
        }

        Layout::compact(false)
    }

    fn layout_fits(&self, layout: Layout, width: u16, height: u16) -> bool {
        self.layout_width(layout) <= width && self.layout_height(layout) <= height
    }

    fn layout_width(&self, layout: Layout) -> u16 {
        match layout.kind {
            LayoutKind::Ascii => {
                if layout.show_seconds {
                    Self::WIDTH
                } else {
                    Self::WIDTH_NO_SECONDS
                }
            }
            LayoutKind::Compact => self.compact_time(layout.show_seconds).len() as u16,
        }
    }

    fn layout_height(&self, layout: Layout) -> u16 {
        match layout.kind {
            LayoutKind::Ascii => {
                Self::ASCII_HEIGHT
                    + if layout.show_footer {
                        Self::FOOTER_HEIGHT
                    } else {
                        0
                    }
            }
            LayoutKind::Compact => 1,
        }
    }

    fn footer_text(&self, max_len: u16) -> Result<String, Error> {
        let mut text = self.mode.text(max_len)?;

        if matches!(self.mode, ClockMode::Time { .. }) && self.use_12h {
            let hour = self.mode.get_time().0;
            text.push_str(if hour < 12 {
                Self::AM_SUFFIX
            } else {
                Self::PM_SUFFIX
            });
        }

        Ok(text)
    }

    fn compact_time(&self, show_seconds: bool) -> String {
        let (mut hour, minute, second) = self.mode.get_time();
        let mut suffix = "";

        if matches!(self.mode, ClockMode::Time { .. }) && self.use_12h {
            suffix = if hour < 12 {
                Self::COMPACT_AM_SUFFIX
            } else {
                Self::COMPACT_PM_SUFFIX
            };

            if hour > 12 {
                hour -= 12;
            } else if hour == 0 {
                hour = 12;
            }
        }

        if show_seconds {
            format!("{hour:02}:{minute:02}:{second:02}{suffix}")
        } else {
            format!("{hour:02}:{minute:02}{suffix}")
        }
    }

    pub fn fmt(&self, w: &mut BufWriter<StdoutLock<'_>>) -> Result<(), Error> {
        if self.layout.kind == LayoutKind::Compact {
            let bold_escape_str = if self.bold { Color::BOLD } else { "" };

            writeln!(
                w,
                "{}{}{bold_escape_str}{}{}",
                self.padding.clock,
                self.color.foreground(),
                self.compact_time(self.layout.show_seconds),
                Color::RESET,
            )?;

            return Ok(());
        }

        let text = self.footer_text(self.layout_width(self.layout))?;
        let (mut hour, minute, second) = self.mode.get_time();

        if matches!(self.mode, ClockMode::Time { .. }) && self.use_12h {
            if hour > 12 {
                hour -= 12;
            } else if hour == 0 {
                hour = 12;
            }
        }

        let color = &self.color;

        for row in 0..5 {
            let colon_character = if self.blink && (second & 1 == 1) {
                Character::Empty
            } else {
                Character::Colon
            };

            let colon = colon_character.fmt(color, row);
            let h0 = Character::Num(hour / 10).fmt(color, row);
            let h1 = Character::Num(hour % 10).fmt(color, row);
            let m0 = Character::Num(minute / 10).fmt(color, row);
            let m1 = Character::Num(minute % 10).fmt(color, row);

            write!(w, "{}{h0}{h1}{colon}{m0}{m1}", self.padding.clock)?;

            if self.layout.show_seconds {
                let s0 = Character::Num(second / 10).fmt(color, row);
                let s1 = Character::Num(second % 10).fmt(color, row);

                write!(w, "{colon}{s0}{s1}")?;
            }

            writeln!(w, "\r")?;
        }

        let bold_escape_str = if self.bold { Color::BOLD } else { "" };

        if self.layout.show_footer {
            writeln!(
                w,
                "\n{bold_escape_str}{}{}{text}",
                self.padding.text,
                self.color.foreground()
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, Layout};
    use crate::{clock::mode::ClockMode, config::Config};

    fn clock() -> Clock {
        Clock::new(Config::default(), ClockMode::default())
    }

    #[test]
    fn uses_smaller_ascii_before_compact_when_width_is_tight() {
        let clock = clock();

        assert_eq!(clock.layout(33, 7), Layout::ascii(false, true));
    }

    #[test]
    fn hides_footer_before_falling_back_to_compact() {
        let clock = clock();

        assert_eq!(clock.layout(52, 5), Layout::ascii(true, false));
    }

    #[test]
    fn drops_seconds_when_full_ascii_width_would_wrap() {
        let clock = clock();

        assert_eq!(clock.layout(51, 7), Layout::ascii(false, true));
    }

    #[test]
    fn falls_back_to_compact_when_short_ascii_width_would_wrap() {
        let clock = clock();

        assert_eq!(clock.layout(32, 7), Layout::compact(true));
    }

    #[test]
    fn falls_back_to_compact_when_ascii_cannot_fit() {
        let clock = clock();

        assert_eq!(clock.layout(10, 1), Layout::compact(true));
    }
}

use std::fmt::Write;

use chrono::{Local, Timelike, Utc};

use crate::error::Error;

pub enum TimeZone {
    Local,
    Utc,
}

impl TimeZone {
    pub fn from_utc(utc: bool) -> Self {
        if utc {
            return Self::Utc;
        }

        Self::Local
    }

    pub fn get_time(&self) -> (u32, u32, u32) {
        if let Self::Utc = self {
            let utc = Utc::now();

            return (utc.hour(), utc.minute(), utc.second());
        }

        let local = Local::now();

        (local.hour(), local.minute(), local.second())
    }

    pub fn text(&self, date_format: &str, max_len: u16) -> Result<String, Error> {
        let mut text = String::new();

        write!(
            text,
            "{}",
            match self {
                Self::Local => Local::now().format(date_format),
                Self::Utc => Utc::now().format(date_format),
            }
        )
        .map_err(|err| Error::DateFormatInvalid {
            fmt: date_format.to_string(),
            err: err.to_string(),
        })?;

        let fmt_len = text.len() as u16;

        if fmt_len > max_len {
            return Err(Error::DateFormatTooWide { fmt_len, max_len });
        }

        Ok(text)
    }
}

use crate::{
    clock::{counter::Counter, time_zone::TimeZone},
    error::Error,
};

pub enum ClockMode {
    Counter(Counter),
    Time {
        time_zone: TimeZone,
        date_format: String,
    },
}

impl ClockMode {
    pub fn get_time(&self) -> (u32, u32, u32) {
        match self {
            Self::Counter(counter) => counter.get_time(),
            Self::Time { time_zone, .. } => time_zone.get_time(),
        }
    }

    pub fn text(&self, max_len: u16) -> Result<String, Error> {
        match self {
            Self::Counter(counter) => Ok(counter.text.to_string()),
            Self::Time {
                time_zone,
                date_format,
            } => time_zone.text(date_format, max_len),
        }
    }
}

impl Default for ClockMode {
    fn default() -> Self {
        Self::Time {
            time_zone: TimeZone::Local,
            date_format: "%d-%m-%Y".to_string(),
        }
    }
}

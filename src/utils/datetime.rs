use glib::{DateTime, TimeZone};
use std::time::SystemTime;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct UnixTime(pub i64);

impl UnixTime {
    pub fn to_glib_date_time(&self) -> DateTime {
        DateTime::from_unix_local(self.0)
    }
    pub fn format(&self, format: &str) -> Option<String> {
        self.to_glib_date_time()
            .format(format)
            .map(|v| v.to_string())
    }
}

impl From<SystemTime> for UnixTime {
    fn from(time: SystemTime) -> Self {
        Self(if time > SystemTime::UNIX_EPOCH {
            time.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        } else {
            -(SystemTime::UNIX_EPOCH
                .duration_since(time)
                .unwrap()
                .as_secs() as i64)
        })
    }
}

impl From<DateTime> for UnixTime {
    fn from(datetime: DateTime) -> Self {
        Self(datetime.to_unix())
    }
}

impl From<exif::DateTime> for UnixTime {
    fn from(datetime: exif::DateTime) -> Self {
        Self::from(DateTime::new(
            &TimeZone::new(
                datetime
                    .offset
                    .map(|offset| {
                        format!(
                            "{}{:02}:{:02}",
                            if offset >= 0 { '+' } else { '-' },
                            offset.abs() / 60,
                            offset.abs() % 60
                        )
                    })
                    .as_deref(),
            ),
            datetime.year as i32,
            datetime.month as i32,
            datetime.day as i32,
            datetime.hour as i32,
            datetime.minute as i32,
            datetime.second as f64 + (datetime.nanosecond.unwrap_or_default() as f64 / 1000000.0),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use regex::RegexBuilder;
    use std::time::Duration;

    #[test]
    fn test_unix_time() {
        let matcher = RegexBuilder::new("^\\d{4}-\\d{2}-\\d{2}-%-\\d{2}:\\d{2}:\\d{2}$")
            .build()
            .unwrap();

        let time = UnixTime::from(SystemTime::now());
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));

        let time = UnixTime::from(SystemTime::UNIX_EPOCH);
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));

        let time = UnixTime::from(
            SystemTime::UNIX_EPOCH
                .checked_sub(Duration::from_secs(1))
                .unwrap(),
        );
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));
    }
}

//! This module provides functionality for working with date and time,
//! including timezone information.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_ulonglong, CStr};
use std::ptr;
use std::time::{Duration, SystemTime};

const ONE_MINUTE: u64 = 60; // 60 seconds
const ONE_HOUR: u64 = 60 * 60; // 60 * 60 seconds

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
    "Nov", "Dec",
];

/// Represents timezone information.
#[derive(Debug)]
pub struct TZInfo {
    hours: u64,
    minutes: u64,
    ahead: bool,
}

/// Represents a date and time with timezone information.
#[derive(Debug)]
pub struct DateTime {
    time: Duration,
    tz: TZInfo,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Tm {
    sec: i32,   /* seconds */
    min: i32,   /* minutes */
    hour: i32,  /* hours */
    mday: i32,  /* day of the month */
    mon: i32,   /* month */
    year: i32,  /* year */
    wday: i32,  /* day of the week */
    yday: i32,  /* day in the year */
    isdst: i32, /* daylight saving time */
}

#[cfg(target_family = "windows")]
#[link(name = "kernel32")]
extern "C" {
    fn ctime(time: *const c_ulonglong) -> *const c_char;
    fn time(time: *const Tm) -> c_ulonglong;
    fn gmtime(timep: *const c_ulonglong) -> *const Tm;
    fn localtime(timep: *const c_ulonglong) -> *const Tm;
    fn mktime(time: *const Tm) -> c_ulonglong;
}

#[cfg(target_family = "unix")]
#[link(name = "c")]
extern "C" {
    fn ctime(time: *const c_ulonglong) -> *const c_char;
    fn time(time: *const Tm) -> c_ulonglong;
    fn gmtime(timep: *const c_ulonglong) -> *const Tm;
    fn localtime(timep: *const c_ulonglong) -> *const Tm;
    fn mktime(time: *const Tm) -> c_ulonglong;
}

impl TZInfo {
    /// Creates a new `TZInfo` instance based on the current system timezone.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it calls C functions to retrieve
    /// timezone information. However the C functions called use only static
    /// memory, so this function should be memory-safe.
    /// The caller must ensure that the system's time and timezone settings are
    /// correctly configured.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::datetime::TZInfo;
    /// let tz_info = unsafe { TZInfo::new() };
    /// println!("Current timezone offset: {}", tz_info.to_str());
    /// ```
    #[expect(clippy::cast_possible_wrap)]
    #[must_use]
    pub unsafe fn new() -> Self {
        // Current TimeStamp
        let ts = time(ptr::null());

        // Local Time
        let local = localtime(std::ptr::from_ref::<u64>(&ts));
        let local_ts = mktime(local);

        // GMT/UTC Time
        let gmt = gmtime(std::ptr::from_ref::<u64>(&ts));
        let mut gmt_ts = mktime(gmt);

        // If GMT is in Daylight Savings, remove subtract an hour
        if (*gmt).isdst > 0 {
            gmt_ts -= ONE_HOUR;
        }

        let diff = (local_ts as i64) - (gmt_ts as i64);
        let ahead = diff >= 0;

        let diff: u64 = diff.unsigned_abs();

        let hours = diff / ONE_HOUR;
        let minutes = (diff % ONE_HOUR) / ONE_MINUTE;

        Self {
            hours,
            minutes,
            ahead,
        }
    }

    /// Converts the timezone information to a string representation.
    ///
    /// The format used is "+hhmm" or "-hhmm" where
    /// - `hh` is a 2 digit representation of difference in hours
    /// - `mm` is a 2 digit representation of difference in minutes
    /// - `+` means local time is ahead of UTC
    /// - `-` means local time is behind UTC
    ///
    /// For example, `"+0230"` would mean local time is 2 hours and 30 minutes
    /// ahead of UTC
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::datetime::TZInfo;
    /// let tz_info = unsafe { TZInfo::new() };
    /// assert!(tz_info.to_str().starts_with(['+', '-']));
    /// ```
    #[must_use]
    pub fn to_str(&self) -> String {
        let mut repr = String::new();

        repr.push(if self.ahead { '+' } else { '-' });
        repr.push_str(format!("{:02}{:02}", self.hours, self.minutes).as_str());

        repr
    }

    /// Creates a new `TZInfo` from a Git timezone string (e.g. "+0530" or "-0800")
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::datetime::TZInfo;
    /// let tz = TZInfo::from_git_string("+0530").unwrap();
    /// assert_eq!(tz.to_str(), "+0530");
    /// ```
    #[must_use]
    pub fn from_git_string(s: &str) -> Option<Self> {
        if s.len() != 5 {
            return None;
        }

        let ahead = &s[0..1] == "+";
        let hours = s[1..3].parse::<u64>().ok()?;
        let minutes = s[3..5].parse::<u64>().ok()?;

        if hours >= 24 || minutes >= 60 {
            return None;
        }

        Some(Self {
            hours,
            minutes,
            ahead,
        })
    }
}

impl DateTime {
    /// Creates a new `DateTime` instance representing the current date and time.
    ///
    /// # Panics
    ///
    /// This function will panic if the system time is set to a date before the Unix epoch
    /// (January 1, 1970). This is extremely unlikely to occur in practice.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::datetime::DateTime;
    ///
    /// let now = DateTime::now();
    /// println!("Current date and time: {}", now.to_str());
    /// ```
    #[must_use]
    pub fn now() -> Self {
        let cur_time = SystemTime::now();
        let time = cur_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards");

        unsafe {
            let tz = TZInfo::new();
            Self { time, tz }
        }
    }

    /// Creates a new `DateTime` instance from a Unix timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::datetime::DateTime;
    ///
    /// let timestamp = 1609459200; // January 1, 2021 00:00:00 UTC
    /// let date_time = DateTime::from_timestamp(timestamp);
    /// println!("Date and time: {}", date_time.to_str());
    /// ```
    #[must_use]
    pub fn from_timestamp(timestamp: u64) -> Self {
        Self {
            time: Duration::from_secs(timestamp),
            tz: unsafe { TZInfo::new() },
        }
    }

    /// Converts the `DateTime` to a string representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::datetime::DateTime;
    ///
    /// let now = DateTime::now();
    /// let date_string = now.to_str();
    /// println!("Current date and time: {}", date_string);
    /// ```
    #[must_use]
    pub fn to_str(&self) -> String {
        let time_str = unsafe {
            let str_time =
                ctime(std::ptr::from_ref::<u64>(&self.time.as_secs()));
            CStr::from_ptr(str_time).to_string_lossy().to_string()
        };

        let mut time_str = time_str
            .split(' ')
            .filter_map(|x| {
                let x = x.trim();
                if x.is_empty() {
                    None
                } else {
                    Some(x)
                }
            })
            .collect::<Vec<&str>>()
            .join(" ");

        time_str.push(' ');
        time_str.push_str(&self.tz.to_str());
        time_str
    }

    /// Creates a new `DateTime` from a Git author/committer timestamp string
    /// Format: "name &lt;email&gt; timestamp timezone"
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::datetime::DateTime;
    /// let dt = DateTime::from_git_timestamp("John Doe <john@example.com> 1234567890 +0000").unwrap();
    /// assert!(dt.to_str().contains("2009"));
    /// ```
    #[expect(clippy::missing_panics_doc)]
    #[must_use]
    pub fn from_git_timestamp(timestamp_str: &str) -> Option<Self> {
        let parts: Vec<&str> = timestamp_str.split_whitespace().collect();

        if parts.len() >= 4 {
            let timestamp = parts[parts.len() - 2].parse::<u64>().ok()?;
            let tz =
                TZInfo::from_git_string(parts.last().expect("Has len > 1"))?;

            // Adjust timestamp for timezone
            let adjusted_timestamp = if tz.ahead {
                timestamp.checked_add(
                    tz.hours * ONE_HOUR + tz.minutes * ONE_MINUTE,
                )?
            } else {
                timestamp.checked_sub(
                    tz.hours * ONE_HOUR + tz.minutes * ONE_MINUTE,
                )?
            };

            let mut dt = Self::from_timestamp(adjusted_timestamp);
            dt.tz = tz;
            Some(dt)
        } else {
            None
        }
    }

    /// Format the date in Git's preferred format (e.g. "Fri Feb 13 23:31:30 2009 +0000")
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::datetime::DateTime;
    /// let dt = DateTime::from_timestamp(1234567890);
    /// assert!(dt.format_git().contains("2009"));
    /// ```
    #[expect(clippy::cast_sign_loss)]
    #[must_use]
    pub fn format_git(&self) -> String {
        unsafe {
            let time_secs = self.time.as_secs();
            let tm = gmtime(std::ptr::from_ref(&time_secs));
            if tm.is_null() {
                return self.to_str();
            }

            let tm = *tm;
            format!(
                "{} {} {:2} {:02}:{:02}:{:02} {} {}",
                WEEKDAYS[tm.wday as usize],
                MONTHS[tm.mon as usize],
                tm.mday,
                tm.hour,
                tm.min,
                tm.sec,
                1900 + tm.year,
                self.tz.to_str()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_tzinfo_new() {
        unsafe {
            let tz = TZInfo::new();
            assert!(tz.hours < 24, "Hours should be less than 24");
            assert!(tz.minutes < 60, "Minutes should be less than 60");
        }
    }

    #[test]
    fn test_tzinfo_to_str() {
        let tz = TZInfo {
            hours: 5,
            minutes: 30,
            ahead: true,
        };
        assert_eq!(tz.to_str(), "+0530");

        let tz = TZInfo {
            hours: 3,
            minutes: 45,
            ahead: false,
        };
        assert_eq!(tz.to_str(), "-0345");
    }

    #[test]
    #[expect(clippy::cast_possible_wrap)]
    fn test_datetime_now() {
        let now = DateTime::now();
        let system_time = SystemTime::now();
        let system_duration = system_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards");

        // Allow for a small difference due to the time it takes to execute the code
        assert!(
            (now.time.as_secs() as i64 - system_duration.as_secs() as i64)
                .abs()
                < 2,
            "DateTime::now() should be close to the current system time"
        );
    }

    #[test]
    fn test_datetime_from_timestamp() {
        let timestamp = 1_609_459_200; // January 1, 2021 00:00:00 UTC
        let dt = DateTime::from_timestamp(timestamp);
        assert_eq!(dt.time.as_secs(), timestamp);
    }

    #[test]
    fn test_tzinfo_debug_impl() {
        let tz = TZInfo {
            hours: 2,
            minutes: 30,
            ahead: true,
        };
        let debug_str = format!("{tz:?}");
        assert!(debug_str.contains("hours: 2"));
        assert!(debug_str.contains("minutes: 30"));
        assert!(debug_str.contains("ahead: true"));
    }

    #[test]
    fn test_tzinfo_from_git_string() {
        let tz = TZInfo::from_git_string("+0530").unwrap();
        assert_eq!(tz.hours, 5);
        assert_eq!(tz.minutes, 30);
        assert!(tz.ahead);
        assert_eq!(tz.to_str(), "+0530");

        let tz = TZInfo::from_git_string("-0800").unwrap();
        assert_eq!(tz.hours, 8);
        assert_eq!(tz.minutes, 0);
        assert!(!tz.ahead);
        assert_eq!(tz.to_str(), "-0800");

        assert!(TZInfo::from_git_string("+2400").is_none());
        assert!(TZInfo::from_git_string("+0060").is_none());
        assert!(TZInfo::from_git_string("invalid").is_none());
    }

    #[test]
    fn test_datetime_from_git_timestamp() {
        let dt = DateTime::from_git_timestamp(
            "John Doe <john@example.com> 1234567890 +0000",
        )
        .unwrap();
        let formatted = dt.format_git();
        dbg!(&formatted);
        assert!(formatted.contains("2009"));
        assert!(formatted.contains("Feb"));
        assert!(formatted.ends_with("+0000"));

        // Test invalid timestamp
        assert!(DateTime::from_git_timestamp("invalid timestamp").is_none());
    }

    #[test]
    fn test_git_format() {
        let dt = DateTime::from_timestamp(1_234_567_890);
        let formatted = dt.format_git();
        assert!(formatted.contains("Feb"));
        assert!(formatted.contains("2009"));
        assert!(formatted.matches(':').count() == 2);
    }
}

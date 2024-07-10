//! This module provides functionality for working with date and time,
//! including timezone information.

use std::ffi::{c_char, c_ulonglong, CStr};
use std::ptr;
use std::time::{Duration, SystemTime};

const ONE_HOUR: u64 = 60 * 60;

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
    /// # use mini_git::datetime::TZInfo;
    /// let tz_info = unsafe { TZInfo::new() };
    /// println!("Current timezone offset: {}", tz_info.to_str());
    /// ```
    #[allow(clippy::cast_possible_wrap)]
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
        let minutes = diff - hours * ONE_HOUR;

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
    /// # use mini_git::datetime::TZInfo;
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
    /// use mini_git::datetime::DateTime;
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
    /// use mini_git::datetime::DateTime;
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
    /// use mini_git::datetime::DateTime;
    ///
    /// let now = DateTime::now();
    /// let date_string = now.to_str();
    /// println!("Current date and time: {}", date_string);
    /// assert!(date_string.ends_with("+0000") || date_string.contains("-"));
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
        let timestamp = 1609459200; // January 1, 2021 00:00:00 UTC
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
        let debug_str = format!("{:?}", tz);
        assert!(debug_str.contains("hours: 2"));
        assert!(debug_str.contains("minutes: 30"));
        assert!(debug_str.contains("ahead: true"));
    }
}

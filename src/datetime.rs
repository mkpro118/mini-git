use std::ffi::{c_char, c_ulonglong, CStr};
use std::ptr;
use std::time::{Duration, SystemTime};

const ONE_HOUR: u64 = 60 * 60;

#[derive(Debug)]
pub struct TZInfo {
    hours: u64,
    minutes: u64,
    ahead: bool,
}

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

    #[must_use]
    pub fn to_str(&self) -> String {
        let mut repr = String::new();

        repr.push(if self.ahead { '+' } else { '-' });
        repr.push_str(format!("{:02}{:02}", self.hours, self.minutes).as_str());

        repr
    }
}

impl DateTime {
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

    #[must_use]
    pub fn from_timestamp(timestamp: u64) -> Self {
        Self {
            time: Duration::from_secs(timestamp),
            tz: unsafe { TZInfo::new() },
        }
    }

    #[must_use]
    pub fn to_str(&self) -> String {
        let time_str = unsafe {
            let str_time = ctime(std::ptr::from_ref::<u64>(&self.time.as_secs()));
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

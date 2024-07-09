#![allow(dead_code)]
use std::ffi::{c_char, c_ulonglong, CStr};
use std::mem::MaybeUninit;
use std::ptr;
use std::time::{Duration, SystemTime};

const ONE_HOUR: u64 = 60 * 60;

#[derive(Debug)]
pub struct TZInfo {
    hours: usize,
    minutes: usize,
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
    tm_sec: i32,   /* seconds */
    tm_min: i32,   /* minutes */
    tm_hour: i32,  /* hours */
    tm_mday: i32,  /* day of the month */
    tm_mon: i32,   /* month */
    tm_year: i32,  /* year */
    tm_wday: i32,  /* day of the week */
    tm_yday: i32,  /* day in the year */
    tm_isdst: i32, /* daylight saving time */
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
    pub unsafe fn new() -> Self {
        // Current TimeStamp
        let ts = time(ptr::null());

        // Local Time
        let local = localtime(&ts as *const u64);
        let local_ts = mktime(local);

        // GMT/UTC Time
        let gmt = gmtime(&ts as *const u64);
        let mut gmt_ts = mktime(gmt);

        // If GMT is in Daylight Savings, remove subtract an hour
        if (*gmt).tm_isdst > 0 {
            gmt_ts = gmt_ts - ONE_HOUR;
        }

        let diff = (local_ts as i64) - (gmt_ts as i64);
        let ahead = diff >= 0;

        let diff: u64 = diff.abs() as u64;

        let hours = diff / ONE_HOUR;
        let minutes = diff - hours * ONE_HOUR;

        Self {
            hours: hours as usize,
            minutes: minutes as usize,
            ahead,
        }
    }

    fn to_str(&self) -> String {
        let mut repr = String::new();

        repr.push(if self.ahead { '+' } else { '-' });
        repr.push_str(format!("{:02}{:02}", self.hours, self.minutes).as_str());

        repr
    }
}

impl DateTime {
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

    pub fn to_str(&self) -> String {
        let time_str = unsafe {
            let str_time = ctime(&self.time.as_secs() as *const u64);
            CStr::from_ptr(str_time).to_string_lossy().to_string()
        };

        let mut time_str = time_str
            .split(" ")
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

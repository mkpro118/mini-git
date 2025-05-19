//! A cross-platform file globbing module.
//!
//! This module provides functionality to perform file globbing (pattern matching for file paths)
//! on both Unix-like systems and Windows. It uses the native globbing functions of each platform
//! for efficient file matching.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! use mini_git::utils::fnmatch::fnmatch;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let matches = fnmatch("*.rs")?;
//!     for file in matches {
//!         println!("Matched file: {}", file);
//!     }
//!     Ok(())
//! }
//! ```

#![allow(unsafe_code)]
#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]

/// Unix-specific globbing implementation.
#[expect(clippy::struct_field_names)]
#[cfg(target_family = "unix")]
pub mod glob {
    use std::error::Error;
    use std::ffi::{c_char, c_int, CStr, CString};
    use std::fs::canonicalize;
    use std::path::PathBuf;
    use std::ptr;

    #[cfg(target_os = "macos")]
    use std::ffi::c_void;

    #[cfg(target_os = "macos")]
    #[repr(C)]
    struct Glob {
        gl_pathc: usize,            // Count of total paths so far.
        gl_matchc: usize,           // Count of paths matching pattern.
        gl_offs: usize,             // Reserved at beginning of gl_pathv.
        gl_flags: c_int,            // Copy of flags parameter to glob.
        gl_pathv: *mut *mut c_char, // List of paths matching pattern.
        // Additional fields specific to macOS
        gl_errfunc:
            Option<extern "C" fn(epath: *const c_char, errno: c_int) -> c_int>,
        gl_closedir: Option<unsafe extern "C" fn(dirp: *mut c_void) -> c_int>,
        gl_readdir:
            Option<unsafe extern "C" fn(dirp: *mut c_void) -> *mut c_void>,
        gl_opendir:
            Option<unsafe extern "C" fn(name: *const c_char) -> *mut c_void>,
        gl_lstat: Option<
            unsafe extern "C" fn(
                pathname: *const c_char,
                buf: *mut c_void,
            ) -> c_int,
        >,
        gl_stat: Option<
            unsafe extern "C" fn(
                pathname: *const c_char,
                buf: *mut c_void,
            ) -> c_int,
        >,
    }

    #[cfg(target_os = "macos")]
    impl Default for Glob {
        fn default() -> Self {
            unsafe { std::mem::zeroed() }
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[repr(C)]
    struct Glob {
        gl_pathc: usize,
        gl_pathv: *mut *mut c_char,
        gl_offs: usize,
        _reserved: [usize; 6],
    }

    #[cfg(not(target_os = "macos"))]
    impl Default for Glob {
        fn default() -> Self {
            Self {
                gl_pathc: 0,
                gl_pathv: ptr::null_mut(),
                gl_offs: 0,
                _reserved: [0usize; 6],
            }
        }
    }

    const GLOB_NOMATCH: c_int = 3;

    #[link(name = "c")]
    extern "C" {
        fn glob(
            pattern: *const c_char,
            flags: c_int,
            errfunc: *mut std::os::raw::c_void,
            pglob: *mut Glob,
        ) -> c_int;
        fn globfree(pglob: *mut Glob);
    }

    /// Performs file globbing on Unix-like systems.
    ///
    /// This function uses the system's `glob` function to find files matching the given pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - A string slice that holds the pattern to match against.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - A vector of strings containing the matched file paths.
    /// * `Err(Box<dyn Error>)` - An error if no matches are found or if an error occurs during globbing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mini_git::utils::fnmatch::fnmatch;
    ///
    /// #[cfg(target_family = "unix")]
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let matches = fnmatch("/home/user/*.rs")?;
    ///     for file in matches {
    ///         println!("Matched Rust file: {}", file);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn fnmatch(pattern: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let pattern = CString::new(pattern)?;
        let mut glob_result = Glob::default();

        let mut paths = vec![];

        unsafe {
            let result =
                glob(pattern.as_ptr(), 0, ptr::null_mut(), &mut glob_result);

            match result {
                0 => {
                    for i in 0..glob_result.gl_pathc {
                        let path = CStr::from_ptr(*glob_result.gl_pathv.add(i));
                        let file_name =
                            String::from_utf8_lossy(path.to_bytes())
                                .to_string();
                        paths.push(
                            canonicalize(PathBuf::from(file_name))
                                .expect("Should be able to get canonical path")
                                .to_str()
                                .expect(
                                    "Should be able to convert path to string",
                                )
                                .to_string(),
                        );
                    }

                    globfree(&mut glob_result);
                    Ok(paths)
                }
                GLOB_NOMATCH => Err("No matches found!".into()),
                _ => Err("An error occurred while globbing.".into()),
            }
        }
    }
}

/// Windows-specific globbing implementation.
#[cfg(target_family = "windows")]
pub mod glob {
    use std::error::Error;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};
    use std::ptr;

    use std::os::raw::c_void;

    type Handle = *mut c_void;
    type Dword = u32;
    type Lpcwstr = *const u16;

    #[repr(C)]
    struct Win32FindDataw {
        dw_file_attributes: Dword,
        ft_creation_time: [Dword; 2],
        ft_last_access_time: [Dword; 2],
        ft_last_write_time: [Dword; 2],
        n_file_size_high: Dword,
        n_file_size_low: Dword,
        dw_reserved0: Dword,
        dw_reserved1: Dword,
        c_file_name: [u16; 260],
        c_alternate_file_name: [u16; 14],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn FindFirstFileW(
            lpFileName: Lpcwstr,
            lpFindFileData: *mut Win32FindDataw,
        ) -> Handle;
        fn FindNextFileW(
            hFindFile: Handle,
            lpFindFileData: *mut Win32FindDataw,
        ) -> i32;
        fn FindClose(hFindFile: Handle) -> i32;
    }

    /// Performs file globbing on Windows systems.
    ///
    /// This function uses the Windows API functions to find files matching the given pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - A string slice that holds the pattern to match against.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - A vector of strings containing the matched file paths.
    /// * `Err(Box<dyn Error>)` - An error if no matches are found or if the pattern is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mini_git::utils::fnmatch::fnmatch;
    ///
    /// #[cfg(target_family = "windows")]
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let matches = fnmatch("C:\\Users\\*.txt")?;
    ///     for file in matches {
    ///         println!("Matched text file: {}", file);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[allow(clippy::cmp_null)]
    pub fn fnmatch(pattern: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let parent = if pattern.contains('\\') {
            Path::new(pattern)
                .parent()
                .expect("Should have a parent")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            ".\\".to_string()
        };
        let mut results = Vec::new();
        let wide_pattern: Vec<u16> = OsString::from(pattern)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut find_data: Win32FindDataw = unsafe { std::mem::zeroed() };

        unsafe {
            let handle = FindFirstFileW(wide_pattern.as_ptr(), &mut find_data);
            if handle == (usize::MAX as *mut c_void) {
                return Err(
                    "Either no files were found, or pattern was invalid!"
                        .into(),
                );
            } else if handle != ptr::null_mut() {
                loop {
                    let file_name = OsString::from_wide(
                        &find_data.c_file_name[..find_data
                            .c_file_name
                            .iter()
                            .position(|&x| x == 0)
                            .unwrap_or(260)],
                    );
                    results.push(format!(
                        "{parent}\\{}",
                        PathBuf::from(&file_name)
                            .to_str()
                            .expect("Should be able to convert path to string")
                    ));

                    if FindNextFileW(handle, &mut find_data) == 0 {
                        break;
                    }
                }

                FindClose(handle);
            }
        }

        Ok(results)
    }
}

pub use glob::*;

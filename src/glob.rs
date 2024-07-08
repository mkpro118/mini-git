use std::error::Error;

#[cfg(target_family = "unix")]
pub mod glob {
    use super::*;
    use std::ffi::CString;
    use std::os::raw::c_void;

    #[repr(C)]
    struct Glob {
        gl_pathc: usize,
        gl_pathv: *mut *mut c_char,
        gl_offs: usize,
        _reserved: [usize; 6],
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

    pub fn fnmatch(pattern: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let pattern = CString::new(pattern)?;
        let mut glob_result = Glob {
            gl_pathc: 0,
            gl_pathv: ptr::null_mut(),
            gl_offs: 0,
            _reserved: [0usize; 6],
        };

        let mut paths = vec![];

        unsafe {
            let result = glob(pattern.as_ptr(), 0, ptr::null_mut(), &mut glob_result);

            match result {
                0 => {
                    for i in 0..glob_result.gl_pathc {
                        let path = CStr::from_ptr(*glob_result.gl_pathv.add(i));
                        paths.push(String::from_utf8_lossy(path.to_bytes()).to_string());
                    }

                    globfree(&mut glob_result);
                    Ok(paths)
                }
                GLOB_NOMATCH => Err("No matches found.".into()),
                _ => Err("An error occurred while globbing.".into()),
            }
        }
    }
}

#[cfg(target_family = "windows")]
pub mod glob {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::ptr;

    use std::os::raw::c_void;

    type HANDLE = *mut c_void;
    type DWORD = u32;
    type LPCWSTR = *const u16;

    #[repr(C)]
    struct Win32FindDataw {
        dw_file_attributes: DWORD,
        ft_creation_time: [DWORD; 2],
        ft_last_access_time: [DWORD; 2],
        ft_last_write_time: [DWORD; 2],
        n_file_size_high: DWORD,
        n_file_size_low: DWORD,
        dw_reserved0: DWORD,
        dw_reserved1: DWORD,
        c_file_name: [u16; 260],
        c_alternate_file_name: [u16; 14],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn FindFirstFileW(lpFileName: LPCWSTR, lpFindFileData: *mut Win32FindDataw) -> HANDLE;
        fn FindNextFileW(hFindFile: HANDLE, lpFindFileData: *mut Win32FindDataw) -> i32;
        fn FindClose(hFindFile: HANDLE) -> i32;
    }

    pub fn fnmatch(pattern: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut results = Vec::new();
        let wide_pattern: Vec<u16> = OsString::from(pattern)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut find_data: Win32FindDataw = unsafe { std::mem::zeroed() };

        unsafe {
            let handle = FindFirstFileW(wide_pattern.as_ptr(), &mut find_data);
            if handle != ptr::null_mut() {
                loop {
                    let file_name = OsString::from_wide(
                        &find_data.c_file_name[..find_data
                            .c_file_name
                            .iter()
                            .position(|&x| x == 0)
                            .unwrap_or(260)],
                    );
                    results.push(
                        PathBuf::from(file_name)
                            .to_str()
                            .expect("Should be able to convert path to string")
                            .to_string(),
                    );

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

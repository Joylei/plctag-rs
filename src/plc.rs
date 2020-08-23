use crate::ffi;
use crate::status;
use crate::DebugLevel;
use crate::Status;
use std::ffi::CString;

/// get library version (major, minor, patch)
pub fn get_version() -> (usize, usize, usize) {
    let major = get_int_attr("version_major", 0);
    let minor = get_int_attr("version_minor", 0);
    let patch = get_int_attr("version_patch", 0);
    (major as usize, minor as usize, patch as usize)
}

/// check plc tag library version
pub fn check_version(major: u32, minor: u32, patch: u32) -> bool {
    let rc = unsafe { ffi::plc_tag_check_lib_version(major as i32, minor as i32, patch as i32) };
    rc == status::PLCTAG_STATUS_OK
}

/// get library attribute
///
/// supported attributes:
/// - debug
/// see `debug::DebugLevel` for valid values
/// - version_major
/// - version_minor
/// - version_patch
#[inline]
pub fn get_int_attr(attr: &str, default: i32) -> i32 {
    let attr = CString::new(attr).unwrap();
    unsafe { ffi::plc_tag_get_int_attribute(0, attr.as_ptr(), default) }
}

/// set library attribute
///
/// supported attributes:
/// - debug
/// see `debug::DebugLevel` for valid values
///
/// # Examples
/// ```
/// use plctag::DebugLevel;
/// use plctag::set_int_attr;
///
/// let level = DebugLevel::Error.value() as i32;
/// let status = set_int_attr("debug", level);
/// ```
#[inline]
pub fn set_int_attr(attr: &str, value: i32) -> Status {
    let attr = CString::new(attr).unwrap();
    let rc = unsafe { ffi::plc_tag_set_int_attribute(0, attr.as_ptr(), value) };
    Status::new(rc)
}

#[inline]
pub fn get_debug_level() -> DebugLevel {
    let level = get_int_attr("debug", 0) as u8;
    level.into()
}

#[inline]
pub fn set_debug_level(debug: DebugLevel) {
    let level: u8 = debug.into();
    unsafe { ffi::plc_tag_set_debug_level(level as i32) };
}

pub use ffi::plc_tag_register_logger as register_logger;

#[inline]
pub fn unregister_logger() {
    unsafe { ffi::plc_tag_unregister_logger() };
}

///Shutting Down the Library
#[inline]
pub fn shutdown() {
    unsafe {
        ffi::plc_tag_shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = get_version();
        assert_eq!(version, (2, 1, 14));
    }

    #[test]
    fn test_debug_level() {
        set_debug_level(DebugLevel::Error);
        let level = get_debug_level();
        assert_eq!(level, DebugLevel::Error);
        set_debug_level(DebugLevel::Info);
        let level = get_debug_level();
        assert_eq!(level, DebugLevel::Info);
    }
}

use crate::{ffi, Result};
use std::ffi::CStr;
use std::fmt;

pub const PLCTAG_STATUS_OK: i32 = ffi::PLCTAG_STATUS_OK as i32;
pub const PLCTAG_STATUS_PENDING: i32 = ffi::PLCTAG_STATUS_PENDING as i32;
pub use ffi::PLCTAG_ERR_TIMEOUT;

/// plc tag error code representations
#[derive(Debug, Copy, Clone)]
pub enum Status {
    /// PLCTAG_STATUS_OK = 0
    Ok,
    /// PLCTAG_STATUS_PENDING = 1
    Pending,
    /// other error codes
    Err(i32),
}

impl Status {
    #[inline]
    pub fn new(rc: i32) -> Self {
        match rc {
            PLCTAG_STATUS_OK => Status::Ok,
            PLCTAG_STATUS_PENDING => Status::Pending,
            _ => Status::Err(rc),
        }
    }

    #[inline]
    pub fn is_ok(&self) -> bool {
        match self {
            Status::Ok => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_err(&self) -> bool {
        match self {
            Status::Err(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_pending(&self) -> bool {
        match self {
            Status::Pending => true,
            _ => false,
        }
    }
    #[inline]
    pub fn is_timeout(&self) -> bool {
        match self {
            Status::Err(ref rc) => *rc == ffi::PLCTAG_ERR_TIMEOUT,
            _ => false,
        }
    }

    #[inline]
    pub fn into_result(&self) -> Result<()> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(*self)
        }
    }

    /// decode status from error code to String
    ///
    /// see `libplctag` for all status code
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::Status;
    ///
    /// let status = Status::Ok;
    /// let msg = status.decode();
    /// assert_eq!(msg, "PLCTAG_STATUS_OK");
    /// ```
    #[inline]
    pub fn decode(&self) -> String {
        let rc = (*self).into();

        unsafe {
            let ptr = ffi::plc_tag_decode_error(rc);
            let msg = CStr::from_ptr(ptr);
            msg.to_string_lossy().to_string()
        }
    }

    #[doc(hidden)]
    #[inline]
    pub(crate) fn err_timeout() -> Self {
        Status::new(ffi::PLCTAG_ERR_TIMEOUT)
    }
}

impl From<i32> for Status {
    #[inline]
    fn from(rc: i32) -> Status {
        Status::new(rc)
    }
}

impl From<Status> for i32 {
    #[inline]
    fn from(status: Status) -> i32 {
        match status {
            Status::Err(ref rc) => *rc,
            Status::Pending => PLCTAG_STATUS_PENDING,
            Status::Ok => PLCTAG_STATUS_OK,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.decode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_ok() {
        let status = Status::Ok;
        let msg = status.decode();
        assert_eq!(msg, "PLCTAG_STATUS_OK");
    }

    #[test]
    fn test_status_pending() {
        let status = Status::Pending;
        let msg = status.decode();
        assert_eq!(msg, "PLCTAG_STATUS_PENDING");
    }
}

// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::{ffi, Result};
use std::ffi::CStr;
use std::fmt;

pub const PLCTAG_STATUS_OK: i32 = ffi::PLCTAG_STATUS_OK as i32;
pub const PLCTAG_STATUS_PENDING: i32 = ffi::PLCTAG_STATUS_PENDING as i32;

/// plc tag error code representations
#[derive(Copy, Clone)]
pub enum Status {
    /// PLCTAG_STATUS_OK = 0
    Ok,
    /// PLCTAG_STATUS_PENDING = 1
    Pending,
    /// other error codes
    Err(i32),
}

impl Status {
    /// create [`Status`] from return code of `libplctag` functions
    #[inline(always)]
    pub fn new(rc: i32) -> Self {
        match rc {
            PLCTAG_STATUS_OK => Status::Ok,
            PLCTAG_STATUS_PENDING => Status::Pending,
            _ => Status::Err(rc),
        }
    }

    /// success or not?
    #[inline(always)]
    pub fn is_ok(&self) -> bool {
        matches!(self, Status::Ok)
    }

    /// has error?
    #[inline(always)]
    pub fn is_err(&self) -> bool {
        matches!(self, Status::Err(_))
    }

    /// has pending operations?
    #[inline(always)]
    pub fn is_pending(&self) -> bool {
        matches!(self, Status::Pending)
    }

    /// is timeout error?
    #[inline(always)]
    pub fn is_timeout(&self) -> bool {
        match self {
            Status::Err(ref rc) => *rc == ffi::PLCTAG_ERR_TIMEOUT,
            _ => false,
        }
    }

    /// into [`Result`]
    #[inline(always)]
    pub fn into_result(self) -> Result<()> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(self)
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
}

impl From<i32> for Status {
    #[inline(always)]
    fn from(rc: i32) -> Status {
        Status::new(rc)
    }
}

impl From<Status> for i32 {
    #[inline(always)]
    fn from(status: Status) -> i32 {
        match status {
            Status::Err(ref rc) => *rc,
            Status::Pending => PLCTAG_STATUS_PENDING,
            Status::Ok => PLCTAG_STATUS_OK,
        }
    }
}

impl fmt::Display for Status {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.decode())
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rc: i32 = (*self).into();
        write!(f, "STATUS {}: {}", &rc, self.decode())
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

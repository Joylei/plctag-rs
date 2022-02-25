// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use core::convert::From;

/// provides debugging output when enabled
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum DebugLevel {
    /// 0 - disables debugging output
    None,
    /// 1 - only output errors. Generally these are fatal to the functioning of the library
    Error,
    /// 2 - outputs warnings such as error found when checking a malformed tag attribute string or when unexpected problems are reported from the PLC
    Warn,
    /// 3 - outputs diagnostic information about the internal calls within the library. Includes some packet dumps
    Info,
    /// 4 - outputs detailed diagnostic information about the code executing within the library including packet dumps
    Detail,
    /// 5 - outputs extremely detailed information. Do not use this unless you are trying to debug detailed information about every mutex lock and release. Will output many lines of output per millisecond. You have been warned!
    Spew,
}

impl From<u8> for DebugLevel {
    #[inline]
    fn from(val: u8) -> DebugLevel {
        match val {
            0 => DebugLevel::None,
            1 => DebugLevel::Error,
            2 => DebugLevel::Warn,
            3 => DebugLevel::Info,
            4 => DebugLevel::Detail,
            5 => DebugLevel::Spew,
            _ => DebugLevel::None,
        }
    }
}

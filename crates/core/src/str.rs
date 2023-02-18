// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use core::ffi::{c_char, CStr};
use core::fmt;
#[cfg(feature = "std")]
use std::{borrow::Cow, ffi::CString};

/// c string repr from
/// - [String]
/// - `&str`
/// - `Cow<str>`
/// - [CString]
/// - [CStr]
/// - `&[u8]` or `&[u8;N]` (null-terminated bytes)
/// - `*const c_char`
#[derive(Debug)]
pub enum AString<'a> {
    /// see [CString]
    #[cfg(feature = "std")]
    CString(CString),
    /// see [CStr]
    CStr(&'a CStr),
    /// c_char ptr
    Ptr(*const c_char),
}

impl AsRef<CStr> for AString<'_> {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl AsRef<str> for AString<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AString<'_> {
    /// as c_char ptr
    #[inline]
    pub fn as_ptr(&self) -> *const c_char {
        match self {
            #[cfg(feature = "std")]
            Self::CString(s) => s.as_ptr(),
            Self::CStr(s) => s.as_ptr(),
            Self::Ptr(p) => *p,
        }
    }

    /// as c str
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        match self {
            #[cfg(feature = "std")]
            AString::CString(s) => s.as_c_str(),
            AString::CStr(s) => s,
            AString::Ptr(ptr) => unsafe { CStr::from_ptr(*ptr) },
        }
    }

    /// as str if valid utf8 string
    #[inline]
    pub fn as_str(&self) -> &str {
        self.as_c_str().to_str().unwrap_or_default()
    }
}

#[cfg(feature = "std")]
impl From<String> for AString<'_> {
    fn from(value: String) -> Self {
        let s = CString::new(value).unwrap();
        AString::CString(s)
    }
}

#[cfg(feature = "std")]
impl From<&str> for AString<'_> {
    fn from(value: &str) -> Self {
        let s = CString::new(value).unwrap();
        AString::CString(s)
    }
}

#[cfg(feature = "std")]
impl From<Cow<'_, str>> for AString<'_> {
    fn from(value: Cow<'_, str>) -> Self {
        let s = match value {
            Cow::Owned(s) => CString::new(s).unwrap(),
            Cow::Borrowed(s) => CString::new(s).unwrap(),
        };
        AString::CString(s)
    }
}

#[cfg(feature = "std")]
impl From<CString> for AString<'_> {
    fn from(value: CString) -> Self {
        AString::CString(value)
    }
}

impl<'a> From<&'a [u8]> for AString<'a> {
    fn from(value: &'a [u8]) -> Self {
        AString::CStr(unsafe { CStr::from_bytes_with_nul_unchecked(value) })
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for AString<'a> {
    fn from(value: &'a [u8; N]) -> Self {
        AString::CStr(unsafe { CStr::from_bytes_with_nul_unchecked(value) })
    }
}

impl<'a> From<&'a CStr> for AString<'a> {
    fn from(value: &'a CStr) -> Self {
        AString::CStr(value)
    }
}

impl From<*const c_char> for AString<'_> {
    fn from(value: *const c_char) -> Self {
        AString::Ptr(value)
    }
}

impl<'a, 'b> PartialEq<AString<'b>> for AString<'a> {
    fn eq(&self, other: &AString<'b>) -> bool {
        self.as_c_str().eq(other.as_c_str())
    }
}

impl PartialEq<&str> for AString<'_> {
    fn eq(&self, other: &&str) -> bool {
        *other == self.as_str()
    }
}

impl PartialEq<&CStr> for AString<'_> {
    fn eq(&self, other: &&CStr) -> bool {
        *other == self.as_c_str()
    }
}

impl Eq for AString<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cstr() {
        let s = AString::from(b"abc\0");
        assert_eq!(s, "abc");
    }

    #[test]
    fn test_cstr2() {
        let s = b"abc\0";
        let s = AString::from(s.as_ref());
        assert_eq!(s, "abc");
    }
}

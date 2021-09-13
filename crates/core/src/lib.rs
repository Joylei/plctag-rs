// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

#[cfg(feature = "event")]
extern crate dyn_clone;
#[cfg(feature = "event")]
extern crate once_cell;
#[cfg(feature = "event")]
extern crate parking_lot;
extern crate plctag_sys;

/// reexports ffi Apis
pub mod ffi {
    pub use plctag_sys::*;
}

pub mod builder;
mod debug;
#[cfg(feature = "event")]
/// event handling
pub mod event;
mod raw;
mod status;
#[cfg(feature = "value")]
mod value;

/// plctag result
pub type Result<T> = std::result::Result<T, Status>;
pub use raw::{RawTag, TagId};
pub use status::Status;

#[cfg(feature = "value")]
pub use value::{Decode, Encode};

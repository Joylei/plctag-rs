// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

#[doc(inline)]
pub use plctag_core::*;
#[cfg(feature = "derive")]
#[doc(inline)]
pub use plctag_derive::{GetValue, SetValue};
#[cfg(feature = "log")]
#[doc(inline)]
pub use plctag_log::*;

#[cfg(feature = "async")]
#[doc(inline)]
pub use plctag_async as futures;

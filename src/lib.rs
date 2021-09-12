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

// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::{RawTag, Result};
use paste::paste;
use std::borrow::Cow;

macro_rules! value_impl {
    ($type: ident) => {
        paste! {
            impl Decode for $type {
                #[inline]
                fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
                    let v = tag.[<get_ $type>](offset)?;
                    Ok(v)
                }

            }
            impl Encode for $type {
                #[inline]
                fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
                    tag.[<set_ $type>](offset, *self)
                }
            }
        }
    };
}

/// this trait abstracts tag value.
/// you can use the trait to implement your UDT.
///
/// # Examples
/// with this trait, you can simply get or set tag value
/// ```rust,ignore
/// use plctag::{RawTag, Encode, Decode};
/// let timeout = 100;//ms
/// let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION
/// let tag = RawTag::new(path, timeout).unwrap();
///
/// //read tag
/// tag.read(timeout).unwrap();
/// let offset = 0;
/// let value:u16 = tag.get_value(offset).unwrap();
/// println!("tag value: {}", value);
///
/// let value = value + 10;
/// tag.set_value(offset, value).unwrap();
///
/// //write tag
/// tag.write(timeout).unwrap();
/// println!("write done!");
/// ```
///
/// # UDT
/// ```rust, ignore
/// use plctag::{RawTag, Decode, Encode}
///
/// // define your UDT
/// #[derive(Default)]
/// struct MyUDT {
///     v1:u16,
///     v2:u16,
/// }
/// impl Decode for MyUDT {
///     fn decode(tag: &RawTag, offset: u32) -> Result<Self>{
///         let v1 = u16::decode(tag, offset)?;
///         let v2 = u16::decode(tag, offset + 2)?;
///         Ok(MyUDT{v1,v2})
///     }
/// }
/// impl Encode for MyUDT {
///     fn encode(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
///         self.v1.encode(tag, offset)?;
///         self.v2.encode(tag, offset+2)?;
///         Ok(())
///     }
/// }
///
/// fn main(){
///     let timeout = 100;//ms
/// let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";// YOUR TAG DEFINITION
///     let tag = RawTag::new(path, timeout).unwrap();
///
///     //read tag
///     tag.read(timeout).unwrap();
///     let offset = 0;
///     let mut value:MyUDT = tag.get_value(offset).unwrap();
///     println!("tag value: {}", value);
///
///     value.v1 = value.v1 + 10;
///     tag.set_value(offset, value).unwrap();
///
///     //write tag
///     tag.write(timeout).unwrap();
///     println!("write done!");
/// }
///
/// ```
///
/// Note:
/// Do not perform expensive operations when you derives [`Decode`] or [`Encode`].

pub trait Decode: Sized {
    /// get value at specified byte offset
    fn decode(tag: &RawTag, offset: u32) -> Result<Self>;

    #[doc(hidden)]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        *place = Decode::decode(tag, offset)?;
        Ok(())
    }
}

/// see `Decode`
pub trait Encode {
    /// set value at specified byte offset
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()>;
}

value_impl!(bool);
value_impl!(i8);
value_impl!(u8);
value_impl!(i16);
value_impl!(u16);
value_impl!(i32);
value_impl!(u32);
value_impl!(i64);
value_impl!(u64);
value_impl!(f32);
value_impl!(f64);

impl<T: Decode> Decode for Option<T> {
    #[inline]
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let v = T::decode(tag, offset)?;
        Ok(Some(v))
    }
}

impl<T: Encode> Encode for Option<T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        if let Some(ref v) = self {
            v.encode(tag, offset)?;
        }
        Ok(())
    }
}

impl<T: Encode> Encode for &T {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

impl<T: Decode + Clone> Decode for Cow<'_, T> {
    #[inline]
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let v = T::decode(tag, offset)?;
        Ok(Cow::Owned(v))
    }
}

impl<T: Encode + Clone> Encode for Cow<'_, T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

/// generic value getter/setter
pub trait ValueExt {
    /// get tag value of `T` that derives [`Decode`]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T>;
    /// set tag value that derives [`Encode`]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()>;
}

impl ValueExt for RawTag {
    #[inline]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        T::decode(self, byte_offset)
    }

    #[inline]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        value.encode(self, byte_offset)
    }
}

impl<V: ValueExt> ValueExt for Box<V> {
    #[inline]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        (**self).get_value(byte_offset)
    }
    #[inline]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        (**self).set_value(byte_offset, value)
    }
}

#![cfg(any(feature = "async", feature = "value"))]

use crate::{RawTag, Result};
use paste::paste;

macro_rules! value_impl {
    ($type: ident) => {
        paste! {
            impl TagValue for $type {

                #[inline]
                fn get_value(&mut self ,tag: &RawTag, offset: u32) -> Result<()> {
                    let v = tag.[<get_ $type>](offset)?;
                    *self = v;
                    Ok(())
                }
                #[inline]
                fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
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
/// use plctag::{RawTag, TagValue, GetValue, SetValue};
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
/// use plctag::{Accessor, TagValue, RawTag}
///
/// // define your UDT
/// #[derive(Default)]
/// struct MyUDT {
///     v1:u16,
///     v2:u16,
/// }
/// impl TagValue for MyUDT {
///     fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
///         self.v1.get_value(tag, offset)?;
///         self.v2.get_value(tag, offset + 2)?;
///         Ok(())
///     }
///
///     fn set_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
///         self.v1.set_value(tag, offset)?;
///         self.v2.set_value(tag, offset+2)?;
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
/// Do not perform expensive operations when you implements `TagValue`.
pub trait TagValue: Default {
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>;

    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()>;
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

#[derive(Default)]
pub struct Bit(bool);

impl From<bool> for Bit {
    fn from(v: bool) -> Bit {
        Bit(v)
    }
}

impl From<Bit> for bool {
    fn from(bit: Bit) -> bool {
        bit.0
    }
}

impl TagValue for Bit {
    #[inline]
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()> {
        let v = tag.get_bit(offset)?;
        *self = Bit(v);
        Ok(())
    }
    #[inline]
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        tag.set_bit(offset, self.0)
    }
}

impl<T: TagValue> TagValue for Option<T> {
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()> {
        let mut v: T = Default::default();
        v.get_value(tag, offset)?;
        *self = Some(v);
        Ok(())
    }

    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        if let Some(ref v) = self {
            v.set_value(tag, offset)?;
        }
        Ok(())
    }
}

/// generic getter/setter based on trait `TagValue`
pub trait Accessor {
    /// get tag value of `T` that implements `TagValue`
    fn get_value<T: TagValue>(&self, byte_offset: u32) -> Result<T>;

    /// set tag value that implements `TagValue`
    fn set_value(&self, byte_offset: u32, value: impl TagValue) -> Result<()>;
}

impl Accessor for RawTag {
    /// get tag value of `T` that implements `TagValue`
    #[inline]
    fn get_value<T: TagValue>(&self, byte_offset: u32) -> Result<T> {
        let mut v = Default::default();
        TagValue::get_value(&mut v, self, byte_offset)?;
        Ok(v)
    }

    /// set tag value that implements `TagValue`
    #[inline]
    fn set_value(&self, byte_offset: u32, value: impl TagValue) -> Result<()> {
        TagValue::set_value(&value, self, byte_offset)
    }
}

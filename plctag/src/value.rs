use std::borrow::Cow;

use crate::{RawTag, Result};
use paste::paste;

macro_rules! value_impl {
    ($type: ident) => {
        paste! {
            impl GetValue for $type {
                #[inline]
                fn get_value(&mut self ,tag: &RawTag, offset: u32) -> Result<()> {
                    let v = tag.[<get_ $type>](offset)?;
                    *self = v;
                    Ok(())
                }

            }
            impl SetValue for $type {
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
/// use plctag::{RawTag, TagValue};
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
/// use plctag::{RawTag, GetValue, SetValue}
///
/// // define your UDT
/// #[derive(Default)]
/// struct MyUDT {
///     v1:u16,
///     v2:u16,
/// }
/// impl GetValue for MyUDT {
///     fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>{
///         self.v1.get_value(tag, offset)?;
///         self.v2.get_value(tag, offset + 2)?;
///         Ok(())
///     }
/// }
/// impl SetValue for MyUDT {
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
/// Do not perform expensive operations when you derives [`GetValue`] or [`SetValue`].

pub trait GetValue {
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()>;
}

pub trait SetValue {
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

impl<T: GetValue + Default> GetValue for Option<T> {
    #[inline]
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()> {
        let mut v: T = Default::default();
        v.get_value(tag, offset)?;
        *self = Some(v);
        Ok(())
    }
}

impl<T: SetValue> SetValue for Option<T> {
    #[inline]
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        if let Some(ref v) = self {
            v.set_value(tag, offset)?;
        }
        Ok(())
    }
}

impl<T: SetValue> SetValue for &T {
    #[inline]
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::set_value(self, tag, offset)
    }
}

impl<T: GetValue + Clone> GetValue for Cow<'_, T> {
    #[inline]
    fn get_value(&mut self, tag: &RawTag, offset: u32) -> Result<()> {
        let v = self.to_mut();
        T::get_value(v, tag, offset)
    }
}

impl<T: SetValue + Clone> SetValue for Cow<'_, T> {
    #[inline]
    fn set_value(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::set_value(self, tag, offset)
    }
}

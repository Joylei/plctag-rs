#![cfg(any(feature = "async", feature = "value"))]

use crate::{RawTag, Result};
use paste::paste;

pub trait Accessor {
    fn get_bit(&self, bit_offset: u32) -> Result<bool>;
    fn set_bit(&self, bit_offset: u32, value: bool) -> Result<()>;
    fn get_bool(&self, byte_offset: u32) -> Result<bool>;
    fn set_bool(&self, byte_offset: u32, value: bool) -> Result<()>;
    fn get_i8(&self, byte_offset: u32) -> Result<i8>;
    fn set_i8(&self, byte_offset: u32, value: i8) -> Result<()>;
    fn get_u8(&self, byte_offset: u32) -> Result<u8>;
    fn set_u8(&self, byte_offset: u32, value: u8) -> Result<()>;
    fn get_i16(&self, byte_offset: u32) -> Result<i16>;
    fn set_i16(&self, byte_offset: u32, value: i16) -> Result<()>;
    fn get_u16(&self, byte_offset: u32) -> Result<u16>;
    fn set_u16(&self, byte_offset: u32, value: u16) -> Result<()>;
    fn get_i32(&self, byte_offset: u32) -> Result<i32>;
    fn set_i32(&self, byte_offset: u32, value: i32) -> Result<()>;
    fn get_u32(&self, byte_offset: u32) -> Result<u32>;
    fn set_u32(&self, byte_offset: u32, value: u32) -> Result<()>;
    fn get_i64(&self, byte_offset: u32) -> Result<i64>;
    fn set_i64(&self, byte_offset: u32, value: i64) -> Result<()>;
    fn get_u64(&self, byte_offset: u32) -> Result<u64>;
    fn set_u64(&self, byte_offset: u32, value: u64) -> Result<()>;
    fn get_f32(&self, byte_offset: u32) -> Result<f32>;
    fn set_f32(&self, byte_offset: u32, value: f32) -> Result<()>;
    fn get_f64(&self, byte_offset: u32) -> Result<f64>;
    fn set_f64(&self, byte_offset: u32, value: f64) -> Result<()>;
}

macro_rules! accessor_impl {
    ($type:ident) => {
        paste! {
            #[inline]
            fn [<get_ $type>](&self, byte_offset: u32) -> Result<$type> {
                self.[<get_ $type>](byte_offset)
            }
            #[inline]
            fn [<set_ $type>](&self, byte_offset: u32, value: $type) -> Result<()> {
                self.[<set_ $type>](byte_offset, value)
            }
        }
    };
}

impl Accessor for RawTag {
    #[inline]
    fn get_bit(&self, bit_offset: u32) -> Result<bool> {
        self.get_bit(bit_offset)
    }
    #[inline]
    fn set_bit(&self, bit_offset: u32, value: bool) -> Result<()> {
        self.set_bit(bit_offset, value)
    }
    accessor_impl!(bool);
    accessor_impl!(i8);
    accessor_impl!(u8);
    accessor_impl!(i16);
    accessor_impl!(u16);
    accessor_impl!(i32);
    accessor_impl!(u32);
    accessor_impl!(i64);
    accessor_impl!(u64);
    accessor_impl!(f32);
    accessor_impl!(f64);
}

macro_rules! value_impl {
    ($type: ident) => {
        paste! {
            impl TagValue for $type {

                #[inline]
                fn get_value(&mut self ,rw: &dyn Accessor, offset: u32) -> Result<()> {
                    let v = rw.[<get_ $type>](offset)?;
                    *self = v;
                    Ok(())
                }
                #[inline]
                fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()> {
                    rw.[<set_ $type>](offset, *self)
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
/// use plctag::{Accessor, TagValue, RawTag, GetValue, SetValue}
///
/// // define your UDT
/// #[derive(Default)]
/// struct MyUDT {
///     v1:u16,
///     v2:u16,
/// }
/// impl TagValue for MyUDT {
///     fn get_value(&mut self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
///         self.v1.get_value(accessor, 0)?;
///         self.v2.get_value(accessor, 2)?;
///         Ok(())
///     }
///
///     fn set_value(&mut self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
///         self.v1.set_value(accessor, 0)?;
///         self.v1.set_value(accessor, 2)?;
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
pub trait TagValue: Default {
    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()>;

    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()>;
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
    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let v = rw.get_bit(offset)?;
        *self = Bit(v);
        Ok(())
    }
    #[inline]
    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        rw.set_bit(offset, self.0)
    }
}

impl<T: TagValue> TagValue for Option<T> {
    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let mut v: T = Default::default();
        v.get_value(rw, offset)?;
        *self = Some(v);
        Ok(())
    }

    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        if let Some(ref v) = self {
            v.set_value(rw, offset)?;
        }
        Ok(())
    }
}

pub trait GetValue {
    fn get_value<T: TagValue>(&self, byte_offset: u32) -> Result<T>;
}

pub trait SetValue {
    fn set_value<T: TagValue>(&self, byte_offset: u32, value: T) -> Result<()>;
}

impl GetValue for RawTag {
    #[inline]
    fn get_value<T: TagValue>(&self, byte_offset: u32) -> Result<T> {
        let mut v = Default::default();
        TagValue::get_value(&mut v, self, byte_offset)?;
        Ok(v)
    }
}

impl SetValue for RawTag {
    #[inline]
    fn set_value<T: TagValue>(&self, byte_offset: u32, value: T) -> Result<()> {
        TagValue::set_value(&value, self, byte_offset)
    }
}

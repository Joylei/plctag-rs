// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use crate::{RawTag, Result};
use paste::paste;
use std::{borrow::Cow, marker::PhantomData, rc::Rc, sync::Arc};

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
/// ```rust,no_run
/// use plctag_core::{RawTag, Encode, Decode, ValueExt};
/// let timeout = 1000;//ms
/// let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION
/// let tag = RawTag::new(path, timeout).unwrap();
///
/// //read tag
/// let status = tag.read(timeout);
/// assert!(status.is_ok());
/// let offset = 0;
/// let value:u16 = tag.get_value(offset).unwrap();
/// println!("tag value: {}", value);
///
/// let value = value + 10;
/// tag.set_value(offset, value).unwrap();
///
/// //write tag
/// let status = tag.write(timeout);
/// assert!(status.is_ok());
/// println!("write done!");
/// ```
///
/// # UDT
/// ```rust,no_run
/// use plctag_core::{RawTag, Decode, Encode, Result, ValueExt};
///
/// // define your UDT
/// #[derive(Default, Debug)]
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
///     fn encode(&self, tag: &RawTag, offset: u32) -> Result<()>{
///         self.v1.encode(tag, offset)?;
///         self.v2.encode(tag, offset+2)?;
///         Ok(())
///     }
/// }
///
/// let timeout = 100;//ms
/// let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";// YOUR TAG DEFINITION
/// let tag = RawTag::new(path, timeout).unwrap();
///
/// //read tag
/// let status = tag.read(timeout);
/// assert!(status.is_ok());
/// let offset = 0;
/// let mut value:MyUDT = tag.get_value(offset).unwrap();
/// println!("tag value: {:?}", value);
///
/// value.v1 += 10;
/// tag.set_value(offset, value).unwrap();
///
/// //write tag
/// let status = tag.write(timeout);
/// assert!(status.is_ok());
/// println!("write done!");
///
/// ```
///
/// Note:
/// Do not perform expensive operations when you derives [`Decode`] or [`Encode`].

pub trait Decode: Sized {
    /// get value at specified byte offset
    fn decode(tag: &RawTag, offset: u32) -> Result<Self>;

    #[doc(hidden)]
    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        *place = Decode::decode(tag, offset)?;
        Ok(())
    }
}

/// see [`Decode`]
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

    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        match place {
            Some(ref mut v) => {
                T::decode_in_place(tag, offset, v)?;
            }
            None => {
                *place = Some(T::decode(tag, offset)?);
            }
        }
        Ok(())
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
    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        let place = place.to_mut();
        T::decode_in_place(tag, offset, place)
    }
}

impl<T: Encode + Clone> Encode for Cow<'_, T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

impl<T: Encode> Encode for Arc<T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

impl<T: Decode> Decode for Arc<T> {
    #[inline]
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let v = T::decode(tag, offset)?;
        Ok(Arc::new(v))
    }
    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        if let Some(place) = Arc::get_mut(place) {
            T::decode_in_place(tag, offset, place)?;
        } else {
            *place = Arc::new(T::decode(tag, offset)?);
        }
        Ok(())
    }
}

impl<T: Encode> Encode for Rc<T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

impl<T: Decode> Decode for Rc<T> {
    #[inline]
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let v = T::decode(tag, offset)?;
        Ok(Rc::new(v))
    }
    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        if let Some(place) = Rc::get_mut(place) {
            T::decode_in_place(tag, offset, place)?;
        } else {
            *place = Rc::new(T::decode(tag, offset)?);
        }
        Ok(())
    }
}

impl<T> Encode for PhantomData<T> {
    #[inline]
    fn encode(&self, _tag: &RawTag, _offset: u32) -> Result<()> {
        Ok(())
    }
}

impl<T> Decode for PhantomData<T> {
    #[inline]
    fn decode(_tag: &RawTag, _offset: u32) -> Result<Self> {
        Ok(Default::default())
    }
}

impl<T: Encode> Encode for Box<T> {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        T::encode(self, tag, offset)
    }
}

impl<T: Decode> Decode for Box<T> {
    #[inline]
    fn decode(tag: &RawTag, offset: u32) -> Result<Self> {
        let v = T::decode(tag, offset)?;
        Ok(Box::new(v))
    }
    #[inline]
    fn decode_in_place(tag: &RawTag, offset: u32, place: &mut Self) -> Result<()> {
        let place = place.as_mut();
        T::decode_in_place(tag, offset, place)
    }
}

impl Encode for &[u8] {
    #[inline]
    fn encode(&self, tag: &RawTag, offset: u32) -> Result<()> {
        let _ = tag.set_bytes(offset, self)?;
        Ok(())
    }
}

/// generic value getter/setter
pub trait ValueExt {
    /// get tag value of `T` that derives [`Decode`]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T>;

    /// get value in place
    fn get_value_in_place<T: Decode>(&self, byte_offset: u32, value: &mut T) -> Result<()> {
        let v = self.get_value(byte_offset)?;
        *value = v;
        Ok(())
    }

    /// set tag value that derives [`Encode`]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()>;
}

impl ValueExt for RawTag {
    #[inline]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        T::decode(self, byte_offset)
    }

    #[inline]
    fn get_value_in_place<T: Decode>(&self, byte_offset: u32, value: &mut T) -> Result<()> {
        T::decode_in_place(self, byte_offset, value)
    }

    #[inline]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        value.encode(self, byte_offset)
    }
}

impl<Tag: ValueExt> ValueExt for &Tag {
    #[inline]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        Tag::get_value(self, byte_offset)
    }

    #[inline]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        Tag::set_value(self, byte_offset, value)
    }
}

impl<Tag: ValueExt> ValueExt for Box<Tag> {
    #[inline]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        (**self).get_value(byte_offset)
    }
    #[inline]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        (**self).set_value(byte_offset, value)
    }
}

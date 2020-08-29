#![cfg(any(feature = "async", feature = "value"))]

use crate::Accessor;
use crate::Result;
use paste::paste;

/// abstract tag value;
/// any type `T` will be read/write by `Tag<T>` should implement the trait
pub trait TagValue: Default {
    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()>;

    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()>;
}

macro_rules! rw_impl {
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

rw_impl!(bool);
rw_impl!(i8);
rw_impl!(u8);
rw_impl!(i16);
rw_impl!(u16);
rw_impl!(i32);
rw_impl!(u32);
rw_impl!(i64);
rw_impl!(u64);
rw_impl!(f32);
rw_impl!(f64);

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

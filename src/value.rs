use crate::Accessor;
use crate::{Result, Status};
use paste::paste;
use std::mem;

/// abstract tag value;
/// any type `T` will be read/write by `Tag<T>` should implement the trait
pub trait TagValue: Default {
    /// size of bytes
    fn tag_size(&self) -> u32;

    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()>;

    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()>;
}

macro_rules! rw_impl {
    ($type: ident) => {
        paste! {
            impl TagValue for $type {
                #[inline]
                fn tag_size(&self) -> u32 {
                    mem::size_of::<$type>() as u32
                }

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
    fn tag_size(&self) -> u32 {
        0
    }
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

impl<T: TagValue> TagValue for &mut [T] {
    #[inline]
    fn tag_size(&self) -> u32 {
        //self.iter().fold(0, |acc, &x| acc + x.tag_size())
        if let Some(ref v) = self.first() {
            v.tag_size() * self.len() as u32
        } else {
            0
        }
    }

    #[inline]
    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let mut pos = offset;
        for v in self.iter_mut() {
            (*v).get_value(rw, pos)?;
            pos += (*v).tag_size();
        }
        Ok(())
    }

    #[inline]
    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let mut pos = offset;
        for v in self.iter() {
            v.set_value(rw, pos)?;
            pos += v.tag_size();
        }
        Ok(())
    }
}

impl<T: TagValue> TagValue for Vec<T> {
    #[inline]
    fn tag_size(&self) -> u32 {
        if let Some(ref v) = self.first() {
            v.tag_size() * self.len() as u32
        } else {
            0
        }
    }

    fn get_value(&mut self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let mut pos = offset;
        for v in self.iter_mut() {
            (*v).get_value(rw, pos)?;
            pos += (*v).tag_size();
        }
        Ok(())
    }

    fn set_value(&self, rw: &dyn Accessor, offset: u32) -> Result<()> {
        let mut pos = offset;
        for v in self.iter() {
            v.set_value(rw, pos)?;
            pos += v.tag_size();
        }
        Ok(())
    }
}

impl<T: TagValue> TagValue for Option<T> {
    fn tag_size(&self) -> u32 {
        let v: T = Default::default();
        v.tag_size()
    }

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

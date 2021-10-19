// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

mod park;
mod rw;

use self::{park::Interest, rw::Operation};
use crate::{Result, TagRef};
use plctag_core::{Decode, Encode, RawTag};

/// get ref of [`RawTag`]
pub trait AsRaw {
    /// get ref of [`RawTag`]
    fn as_raw(&self) -> &RawTag;
}

/// async tag
#[async_trait]
pub trait AsyncTag: AsRaw {
    /// get tag id
    #[inline(always)]
    fn id(&self) -> i32 {
        self.as_raw().id()
    }

    /// get tag size in bytes
    #[inline(always)]
    fn size(&self) -> Result<u32> {
        Ok(self.as_raw().size()?)
    }

    /// set tag size
    #[inline(always)]
    fn set_size(&self, size: u32) -> Result<u32> {
        Ok(self.as_raw().set_size(size)?)
    }

    /// element count of this tag
    #[inline(always)]
    fn elem_count(&self) -> Result<i32> {
        Ok(self.as_raw().elem_count()?)
    }

    /// element size
    #[inline(always)]
    fn elem_size(&self) -> Result<i32> {
        Ok(self.as_raw().elem_size()?)
    }

    /// get value from mem, you should call read() before this operation
    #[inline(always)]
    fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        Ok(self.as_raw().get_value(byte_offset)?)
    }

    /// set value in mem, you should call write() later
    #[inline(always)]
    fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        self.as_raw().set_value(byte_offset, value)?;
        Ok(())
    }

    /// perform read from PLC Controller
    #[inline(always)]
    async fn read(&self) -> Result<()> {
        let mut task = Operation::new(self.as_raw(), Interest::Read);
        task.run().await
    }

    /// perform write to PLC Controller
    #[inline(always)]
    async fn write(&self) -> Result<()> {
        let mut task = Operation::new(self.as_raw(), Interest::Write);
        task.run().await
    }

    /// perform read & returns the value
    #[inline(always)]
    async fn read_value<T: Decode>(&self, offset: u32) -> Result<T> {
        self.read().await?;
        Ok(self.as_raw().get_value(offset)?)
    }

    /// set the value and write to PLC Controller
    #[inline(always)]
    async fn write_value<T: Encode + Send>(&self, offset: u32, value: T) -> Result<()> {
        self.as_raw().set_value(offset, value)?;
        self.write().await?;
        Ok(())
    }
}
impl AsRaw for TagRef<'_> {
    #[inline(always)]
    fn as_raw(&self) -> &RawTag {
        &self.tag
    }
}
impl AsyncTag for TagRef<'_> {}

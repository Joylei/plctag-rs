use std::time::Duration;

use crate::{cell::SyncCell, Result, TagRef};
use plctag::{event::Event, Accessor, RawTag, TagValue};

pub trait AsyncTagBase {
    fn get_tag(&self) -> &RawTag;
}
pub trait AsyncTag: AsyncTagBase {
    #[inline(always)]
    fn size(&self) -> Result<u32> {
        let tag = self.get_tag();
        Ok(tag.size()?)
    }
    #[inline(always)]
    fn elem_size(&self) -> Result<i32> {
        let tag = self.get_tag();
        Ok(tag.elem_size()?)
    }
    #[inline(always)]
    fn elem_count(&self) -> Result<i32> {
        let tag = self.get_tag();
        Ok(tag.elem_count()?)
    }

    #[inline(always)]
    fn read(&self, timeout: Option<Duration>) -> Result<()> {
        let mut op = Operation::new(&self.get_tag());
        op.run(true, timeout)
    }

    #[inline(always)]
    fn write(&self, timeout: Option<Duration>) -> Result<()> {
        let mut op = Operation::new(&self.get_tag());
        op.run(false, timeout)
    }

    #[inline(always)]
    fn read_value<T: TagValue + Send + 'static>(
        &self,
        offset: u32,
        timeout: Option<Duration>,
    ) -> Result<T> {
        self.read(timeout)?;
        self.get_value(offset)
    }

    #[inline(always)]
    fn write_value(
        &self,
        offset: u32,
        value: impl TagValue + Send + 'static,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.set_value(offset, value)?;
        self.write(timeout)
    }

    #[inline(always)]
    fn get_value<T: TagValue>(&self, offset: u32) -> Result<T> {
        let tag = self.get_tag();
        let v = tag.get_value(offset)?;
        Ok(v)
    }
    #[inline(always)]
    fn set_value(&self, offset: u32, value: impl TagValue + Send + 'static) -> Result<()> {
        let tag = self.get_tag();
        tag.set_value(offset, value)?;
        Ok(())
    }
}

impl AsyncTagBase for TagRef<'_, RawTag> {
    #[inline(always)]
    fn get_tag(&self) -> &RawTag {
        self.tag
    }
}

impl AsyncTag for TagRef<'_, RawTag> {}

/// ensures that pending operation get aborted if not successful
struct Operation<'a> {
    /// should abort or not
    pending: bool,
    tag: &'a RawTag,
}

impl<'a> Operation<'a> {
    #[inline(always)]
    fn new(tag: &'a RawTag) -> Self {
        Self {
            pending: false,
            tag,
        }
    }

    fn run(&mut self, rw: bool, timeout: Option<Duration>) -> Result<()> {
        let cell = SyncCell::new();
        let handler = {
            let cell = cell.clone();
            self.tag.listen(move |_, evt, status| {
                //TODO: check status
                match evt {
                    Event::ReadCompleted if rw => (),
                    Event::WriteCompleted if !rw => (),
                    Event::Aborted | Event::Destroyed => (),
                    _ => return,
                }
                cell.set(status);
            })
        };
        self.pending = true;
        let mut status = if rw {
            self.tag.read(0)
        } else {
            self.tag.write(0)
        };
        if status.is_pending() {
            status = cell.wait(timeout)?;
            debug_assert!(!status.is_pending());
        }
        self.pending = false;
        drop(handler); //remove listener here
        status.into_result()?;
        Ok(())
    }
}

/// drop ensures that pending operation get aborted if not successful
impl Drop for Operation<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        if self.pending {
            let _ = self.tag.abort();
        }
    }
}

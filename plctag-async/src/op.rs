use std::sync::Arc;

use crate::{cell::OnceCell, Result, TagRef};
use plctag::{event::Event, GetValue, RawTag, SetValue};

pub trait AsyncTagBase {
    fn get_tag(&self) -> &RawTag;
}

#[async_trait]
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
    async fn read(&self) -> Result<()> {
        let mut op = Operation::new(&self.get_tag());
        op.run(true).await
    }

    #[inline(always)]
    async fn write(&self) -> Result<()> {
        let mut op = Operation::new(&self.get_tag());
        op.run(false).await
    }

    #[inline(always)]
    async fn read_value<T: GetValue + Default>(&self, offset: u32) -> Result<T> {
        self.read().await?;
        self.get_value(offset)
    }

    #[inline(always)]
    async fn write_value<T: SetValue + Send>(&self, offset: u32, value: T) -> Result<()> {
        self.set_value(offset, value)?;
        self.write().await?;
        Ok(())
    }

    #[inline(always)]
    fn get_value<T: GetValue + Default>(&self, offset: u32) -> Result<T> {
        let tag = self.get_tag();
        let v = tag.get_value(offset)?;
        Ok(v)
    }

    #[inline(always)]
    fn set_value<T: SetValue>(&self, offset: u32, value: T) -> Result<()> {
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

    async fn run(&mut self, rw: bool) -> Result<()> {
        let cell = Arc::new(OnceCell::new());
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
            status = cell.take().await;
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

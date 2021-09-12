use std::sync::Arc;

use crate::{cell::OnceCell, Result, TagRef};
use plctag_core::{event::Event, GetValue, RawTag, SetValue, TagId};

pub trait AsRaw {
    fn as_raw(&self) -> &RawTag;
}

#[async_trait]
pub trait AsyncTag: AsRaw {
    #[inline(always)]
    fn id(&self) -> TagId {
        self.as_raw().id()
    }

    #[inline(always)]
    fn size(&self) -> Result<u32> {
        Ok(self.as_raw().size()?)
    }

    #[inline(always)]
    fn elem_count(&self) -> Result<i32> {
        Ok(self.as_raw().elem_count()?)
    }

    #[inline(always)]
    fn elem_size(&self) -> Result<i32> {
        Ok(self.as_raw().elem_size()?)
    }

    #[inline(always)]
    fn get_value<T: GetValue + Default>(&self, byte_offset: u32) -> Result<T> {
        Ok(self.as_raw().get_value(byte_offset)?)
    }

    #[inline(always)]
    fn set_value<T: SetValue>(&self, byte_offset: u32, value: T) -> Result<()> {
        self.as_raw().set_value(byte_offset, value)?;
        Ok(())
    }

    #[inline(always)]
    async fn read(&self) -> Result<()> {
        let mut op = Operation::new(self.as_raw());
        op.run(true).await
    }

    #[inline(always)]
    async fn write(&self) -> Result<()> {
        let mut op = Operation::new(self.as_raw());
        op.run(false).await
    }

    #[inline(always)]
    async fn read_value<T: GetValue + Default>(&self, offset: u32) -> Result<T> {
        self.read().await?;
        Ok(self.as_raw().get_value(offset)?)
    }

    #[inline(always)]
    async fn write_value<T: SetValue + Send>(&self, offset: u32, value: T) -> Result<()> {
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
                let _ = cell.set(status);
            })
        };
        self.pending = true;
        let mut status = if rw {
            self.tag.read(0)
        } else {
            self.tag.write(0)
        };
        if status.is_pending() {
            status = *cell.wait().await;
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

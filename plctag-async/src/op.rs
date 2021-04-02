use std::sync::Arc;

use crate::{cell::OnceCell, Result, TagRef};
use plctag::{event::Event, GetValue, RawTag, SetValue};

#[async_trait]
pub trait AsyncTag: AsRef<RawTag> {
    #[inline(always)]
    async fn read(&self) -> Result<()> {
        let mut op = Operation::new(self.as_ref());
        op.run(true).await
    }

    #[inline(always)]
    async fn write(&self) -> Result<()> {
        let mut op = Operation::new(self.as_ref());
        op.run(false).await
    }

    #[inline(always)]
    async fn read_value<T: GetValue + Default>(&self, offset: u32) -> Result<T> {
        self.read().await?;
        Ok(self.as_ref().get_value(offset)?)
    }

    #[inline(always)]
    async fn write_value<T: SetValue + Send>(&self, offset: u32, value: T) -> Result<()> {
        self.as_ref().set_value(offset, value)?;
        self.write().await?;
        Ok(())
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

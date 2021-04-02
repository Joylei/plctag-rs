use std::time::Duration;

use crate::{cell::SyncCell, Result, TagRef};
use plctag::{event::Event, GetValue, RawTag, SetValue};

pub trait AsyncTag: AsRef<RawTag> {
    #[inline(always)]
    fn read(&self, timeout: Option<Duration>) -> Result<()> {
        let mut op = Operation::new(self.as_ref());
        op.run(true, timeout)
    }

    #[inline(always)]
    fn write(&self, timeout: Option<Duration>) -> Result<()> {
        let mut op = Operation::new(self.as_ref());
        op.run(false, timeout)
    }

    #[inline(always)]
    fn read_value<T: GetValue + Default>(
        &self,
        offset: u32,
        timeout: Option<Duration>,
    ) -> Result<T> {
        self.read(timeout)?;
        Ok(self.as_ref().get_value(offset)?)
    }

    #[inline(always)]
    fn write_value<T: SetValue>(
        &self,
        offset: u32,
        value: T,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.as_ref().set_value(offset, value)?;
        self.write(timeout)
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

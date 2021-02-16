use std::{sync::Arc, time::Duration};

use crate::*;

use mailbox::Token;
use may::sync::Blocker;
use once_cell::sync::OnceCell;
use plctag::event::Event;
pub struct TagEntry<O: TagOptions> {
    opts: O,
    token: Token,
}

impl<O: TagOptions> TagEntry<O> {
    #[inline(always)]
    pub(crate) fn new(opts: O, token: Token) -> Self {
        Self { opts, token }
    }

    #[inline(always)]
    pub fn key(&self) -> &str {
        self.opts.key()
    }

    /// tag value size in bytes
    #[inline(always)]
    pub fn size(&self) -> Result<u32> {
        let tag = self.token.get()?;
        let v = tag.size()?;
        Ok(v)
    }
    #[inline(always)]
    pub fn elem_size(&self) -> Result<i32> {
        let tag = self.token.get()?;
        let v = tag.get_attr("elem_size", 0)?;
        Ok(v)
    }
    #[inline(always)]
    pub fn elem_count(&self) -> Result<i32> {
        let tag = self.token.get()?;
        let v = tag.get_attr("elem_count", 0)?;
        Ok(v)
    }

    /// wait until connected or timeout; returns true if connected, false if timeout
    #[inline(always)]
    pub fn connect(&self, timeout: Option<Duration>) -> bool {
        self.token.wait(timeout)
    }

    /// read from plc to memory
    #[inline]
    pub fn read(&self) -> Result<()> {
        let mut op = Operation::new(&self.token);
        op.run(true)
    }

    /// write to plc
    #[inline]
    pub fn write(&self) -> Result<()> {
        let mut op = Operation::new(&self.token);
        op.run(false)
    }

    /// read from plc and returns the value
    #[inline]
    pub fn read_value<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        self.read()?;
        self.get_value(offset)
    }

    /// write the specified value to plc
    #[inline]
    pub fn write_value(&self, offset: u32, value: impl TagValue + Send + 'static) -> Result<()> {
        self.set_value(offset, value)?;
        self.write()
    }

    /// get value in memory
    #[inline(always)]
    pub fn get_value<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        let tag = self.token.get()?;
        let mut v: T = Default::default();
        v.get_value(tag, offset)?;
        Ok(v)
    }

    /// set value  in memory
    #[inline(always)]
    pub fn set_value(&self, offset: u32, value: impl TagValue + Send + 'static) -> Result<()> {
        let tag = self.token.get()?;
        value.set_value(tag, offset)?;
        Ok(())
    }
}

struct Operation<'a> {
    status: Status,
    token: &'a Token,
}

impl<'a> Operation<'a> {
    #[inline(always)]
    fn new(token: &'a Token) -> Self {
        Self {
            status: Status::Ok,
            token,
        }
    }

    fn run(&mut self, rw: bool) -> Result<()> {
        struct Parker {
            cell: Arc<OnceCell<Status>>,
            blocker: Arc<Blocker>,
        }

        impl Parker {
            #[inline(always)]
            fn new() -> Self {
                let blocker = Blocker::current();
                let cell = Arc::new(OnceCell::new());
                Self { cell, blocker }
            }
            #[inline(always)]
            fn park(&self) -> Result<Status> {
                self.blocker.park(None)?;
                let v = self.cell.get().unwrap();
                Ok(*v)
            }
            #[inline(always)]
            fn unpark(&self, val: Status) {
                if self.cell.set(val).is_ok() {
                    self.blocker.unpark();
                }
            }
        }

        impl Clone for Parker {
            #[inline(always)]
            fn clone(&self) -> Self {
                Self {
                    cell: Arc::clone(&self.cell),
                    blocker: Arc::clone(&self.blocker),
                }
            }
        }

        let tag = self.token.get()?;
        self.status = Status::Pending;
        let parker = Parker::new();
        let removal = {
            let parker = parker.clone();
            tag.listen(move |evt, status| {
                //TODO: check status
                match evt {
                    Event::ReadCompleted if rw => (),
                    Event::WriteCompleted if !rw => (),
                    Event::Aborted => (),
                    Event::Destroyed => (),
                    _ => return,
                }
                //interested
                parker.unpark(status.into());
            })
            .manual(false)
            .on()
        };
        let status = if rw { tag.read(0) } else { tag.write(0) };
        self.status = status;
        if status.is_err() {
            status.into_result()?;
        } else if status.is_pending() {
            let status = parker.park()?;
            self.status = status;
            status.into_result()?;
        }
        drop(removal); //remove listener here
        Ok(())
    }
}

/// drop ensures that pending operation get aborted if not successful
impl Drop for Operation<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        if self.status.is_pending() {
            if let Ok(tag) = self.token.get() {
                let _ = tag.abort();
            }
        }
    }
}

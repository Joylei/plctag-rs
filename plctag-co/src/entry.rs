use std::sync::{Arc, Weak};

use crate::*;

use mailbox::{Mailbox, Token};
use may::sync::Blocker;
use once_cell::sync::OnceCell;
use plctag_sys as ffi;

pub struct TagEntry<O: TagOptions> {
    opts: O,
    token: Token,
}

impl<O: TagOptions> TagEntry<O> {
    #[inline(always)]
    pub fn new(opts: O) -> Self {
        lazy_static! {
            static ref MAILBOX: Mailbox = Mailbox::new();
        };
        let token = MAILBOX.create_tag(opts.to_string());
        Self { opts, token }
    }

    #[inline(always)]
    pub fn key(&self) -> &str {
        self.opts.key()
    }

    /// tag value size in bytes
    pub fn size(&self) -> Result<u32> {
        let tag = self.token.get()?;
        let v = tag.size()?;
        Ok(v)
    }

    pub fn elem_size(&self) -> Result<i32> {
        let tag = self.token.get()?;
        let v = tag.get_attr("elem_size", 0)?;
        Ok(v)
    }

    pub fn elem_count(&self) -> Result<i32> {
        let tag = self.token.get()?;
        let v = tag.get_attr("elem_count", 0)?;
        Ok(v)
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
        let tag = self.token.get()?;
        self.status = Status::Pending;
        let blocker = Blocker::current();
        let cell = Arc::new(OnceCell::new());
        let removal = {
            let blocker = Arc::clone(&blocker);
            let cell = Arc::clone(&cell);
            tag.listen(move |evt, status| {
                //TODO: check status
                if evt > 0 {
                    match evt as u32 {
                        ffi::PLCTAG_EVENT_READ_COMPLETED if rw => (),
                        ffi::PLCTAG_EVENT_WRITE_COMPLETED if !rw => (),
                        ffi::PLCTAG_EVENT_ABORTED => (),
                        ffi::PLCTAG_EVENT_DESTROYED => (),
                        _ => return,
                    }
                } else {
                    return;
                }
                //interested
                if cell.set(status).is_ok() {
                    blocker.unpark();
                }
            })
            .manual(false)
            .on()
        };
        let status = if rw { tag.read(0) } else { tag.write(0) };
        self.status = status;
        if status.is_err() {
            status.into_result()?;
        } else if status.is_pending() {
            blocker.park(None)?;
            let status = *cell.get().unwrap();
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

// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::*;
use parking_lot::Mutex;
use plctag_core::Decode;
use plctag_core::Encode;
use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::hint;
use std::mem;
use std::ops::Deref;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Weak;
use tokio::sync::mpsc;
use tokio::sync::Notify;

use plctag_core::ffi::PLCTAG_ERR_NOT_FOUND;
const PLCTAG_EVENT_CREATED: i32 = plctag_core::ffi::PLCTAG_EVENT_CREATED as i32;
const PLCTAG_EVENT_READ_COMPLETED: i32 = plctag_core::ffi::PLCTAG_EVENT_READ_COMPLETED as i32;
const PLCTAG_EVENT_WRITE_COMPLETED: i32 = plctag_core::ffi::PLCTAG_EVENT_WRITE_COMPLETED as i32;
const PLCTAG_EVENT_DESTROYED: i32 = plctag_core::ffi::PLCTAG_EVENT_DESTROYED as i32;

const TAG_CRATED: u8 = 1;
const TAG_FIRST_READ: u8 = 2;
const TAG_DESTROYED: u8 = 3;

/// tag entry, represents a tag in PLC controller
#[derive(Debug)]
pub struct TagEntry {
    tag: RawTag,
    inner: Arc<Inner>,
    _guard: ArcGuard<Inner>,
}

#[derive(Debug)]
struct Inner {
    state: AtomicU8,
    event: Mutex<(i32, i32)>,
    notify: Notify,
}

impl Inner {
    fn new() -> Self {
        Self {
            state: Default::default(),
            event: Default::default(),
            notify: Notify::new(),
        }
    }

    #[inline]
    fn state(&self) -> u8 {
        self.state.load(Ordering::Acquire)
    }

    #[inline]
    fn take_event(&self) -> (i32, i32) {
        let event = &mut *self.event.lock();
        mem::take(event)
    }

    #[inline]
    fn set_event(&self, event: (i32, i32)) -> bool {
        match event.0 {
            PLCTAG_EVENT_CREATED => {
                //dbg!("TAG_CREATED");
                self.state.store(TAG_FIRST_READ, Ordering::Release);
            }
            PLCTAG_EVENT_READ_COMPLETED => {
                // somehow, the read completed event is not handled gracefully by libplctag;
                // so hack it here, swallow the first read completed event;
                // not sure if it's working for modbus?
                if self
                    .state
                    .compare_exchange(
                        TAG_CRATED,
                        TAG_FIRST_READ,
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return false;
                }
            }
            PLCTAG_EVENT_DESTROYED => {
                self.state.store(TAG_DESTROYED, Ordering::Release);
            }
            _ => {}
        }
        {
            let dest = &mut *self.event.lock();
            //dbg!(&dest);
            *dest = event;
        }

        true
    }
}

#[derive(Debug)]
struct ArcGuard<T> {
    ptr: *const T,
}

impl<T> Drop for ArcGuard<T> {
    fn drop(&mut self) {
        unsafe {
            Arc::from_raw(self.ptr);
        }
    }
}

unsafe impl<T> Send for ArcGuard<T> {}
unsafe impl<T> Sync for ArcGuard<T> {}

impl TagEntry {
    /// create instance of [`TagEntry`]
    pub async fn create(options: impl Into<String>) -> Result<Self> {
        extern "C" fn on_event(_tag: i32, event: i32, status: i32, user_data: *mut c_void) {
            match event {
                PLCTAG_EVENT_CREATED
                | PLCTAG_EVENT_DESTROYED
                | PLCTAG_EVENT_READ_COMPLETED
                | PLCTAG_EVENT_WRITE_COMPLETED => unsafe {
                    let ptr = user_data as *const Inner;
                    Arc::increment_strong_count(ptr);
                    let inner = Arc::from_raw(ptr);
                    if inner.set_event((event, status)) {
                        inner.notify.notify_one();
                    }
                },
                _ => {}
            }
        }

        let inner = Arc::new(Inner::new());
        let path = options.into();
        let guard = ArcGuard {
            ptr: Arc::into_raw(inner.clone()),
        };
        let tag = {
            let user_data = guard.ptr as *mut Inner as *mut c_void;
            unsafe { RawTag::new_with_callback(path, 0, Some(on_event), user_data) }?
        };
        // workaround for created event not triggered
        // if !tag.status().is_pending() {
        //     inner.state.store(TAG_FIRST_READ, Ordering::Release);
        // }
        Ok(Self {
            tag,
            inner,
            _guard: guard,
        })
    }

    /// wait until interested event is received
    #[inline]
    async fn recv_event(&self, event: i32) -> Status {
        loop {
            match self.inner.state() {
                TAG_DESTROYED => {
                    return Status::Err(PLCTAG_ERR_NOT_FOUND);
                }
                TAG_FIRST_READ => {
                    let (evt, status) = self.inner.take_event();
                    if evt == event {
                        return Status::from(status);
                    }
                }
                _ => {}
            }
            self.inner.notify.notified().await;
        }
    }

    /// wait until created
    #[inline]
    pub async fn ready(&mut self) -> Result<()> {
        loop {
            match self.inner.state() {
                TAG_DESTROYED => {
                    return Err(Status::Err(PLCTAG_ERR_NOT_FOUND).into());
                }
                TAG_CRATED | TAG_FIRST_READ => {
                    return Ok(());
                }
                _ => {}
            }
            self.inner.notify.notified().await;
        }
    }

    #[inline]
    pub async fn read(&mut self) -> Result<()> {
        self.read_or_write(PLCTAG_EVENT_READ_COMPLETED).await
    }

    #[inline]
    pub async fn write(&mut self) -> Result<()> {
        self.read_or_write(PLCTAG_EVENT_WRITE_COMPLETED).await
    }

    #[inline]
    async fn read_or_write(&mut self, event: i32) -> Result<()> {
        self.ready().await?;
        let mut guard = InflightGuard {
            tag: &self.tag,
            pending: true,
        };
        match event {
            PLCTAG_EVENT_WRITE_COMPLETED => {
                guard.write()?;
            }
            PLCTAG_EVENT_READ_COMPLETED => {
                guard.read()?;
            }
            _ => unreachable!(),
        }

        // pending
        let status = self.recv_event(event).await;
        if status.is_err() {
            guard.pending = false;
            return Err(status.into());
        }
        drop(guard);
        Ok(())
    }

    /// poll status
    #[inline]
    pub async fn status(&mut self) -> Status {
        match self.inner.state() {
            TAG_DESTROYED => Status::Err(PLCTAG_ERR_NOT_FOUND),
            _ => self.tag.status(),
        }
    }

    #[inline]
    pub async fn get_attr(&mut self, attr: impl AsRef<str>, default_value: i32) -> Result<i32> {
        let v = self.tag.get_attr(attr, default_value)?;
        Ok(v)
    }

    #[inline]
    pub async fn set_attr(&mut self, attr: impl AsRef<str>, value: i32) -> Result<()> {
        self.tag.set_attr(attr, value)?;
        Ok(())
    }

    #[inline]
    pub async fn elem_size(&mut self) -> Result<i32> {
        let v = self.tag.elem_size()?;
        Ok(v)
    }

    #[inline]
    pub async fn elem_count(&mut self) -> Result<i32> {
        let v = self.tag.elem_count()?;
        Ok(v)
    }

    #[inline]
    pub async fn size(&mut self) -> Result<u32> {
        let v = self.tag.size()?;
        Ok(v)
    }

    #[inline]
    pub async fn set_size(&mut self, size: u32) -> Result<u32> {
        let v = self.tag.set_size(size)?;
        Ok(v)
    }

    /// get value from mem, you should call read() before this operation
    #[inline]
    pub async fn get_value<T: Decode>(&mut self, byte_offset: u32) -> Result<T> {
        let v = self.tag.get_value(byte_offset)?;
        Ok(v)
    }

    /// set value in mem, you should call write() later
    #[inline]
    pub async fn set_value<T: Encode>(&mut self, byte_offset: u32, value: T) -> Result<()> {
        self.tag.set_value(byte_offset, value)?;
        Ok(())
    }

    /// perform read & returns the value
    #[inline]
    pub async fn read_value<T: Decode>(&mut self, offset: u32) -> Result<T> {
        self.read().await?;
        //dbg!("read done", self.tag.status());
        Ok(self.tag.get_value(offset)?)
    }

    /// set the value and write to PLC Controller
    #[inline]
    pub async fn write_value<T: Encode + Send>(&mut self, offset: u32, value: T) -> Result<()> {
        self.ready().await?;
        self.tag.set_value(offset, value)?;
        self.write().await?;
        Ok(())
    }

    /// get raw bytes
    #[inline]
    pub async fn get_bytes(&mut self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        let v = self.tag.get_bytes(byte_offset, buf)?;
        Ok(v)
    }

    /// set raw bytes
    #[inline]
    pub async fn set_bytes(&mut self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        let v = self.tag.set_bytes(byte_offset, buf)?;
        Ok(v)
    }
}

struct InflightGuard<'a> {
    tag: &'a RawTag,
    pending: bool,
}

impl InflightGuard<'_> {
    #[inline]
    fn read(&mut self) -> Result<()> {
        let status = self.tag.read(0);
        match status {
            Status::Pending => {
                self.pending = true;
                Ok(())
            }
            Status::Err(_) => {
                self.pending = false;
                Err(status.into())
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn write(&mut self) -> Result<()> {
        let status = self.tag.write(0);
        match status {
            Status::Pending => {
                self.pending = true;
                Ok(())
            }
            Status::Err(_) => {
                self.pending = false;
                Err(status.into())
            }
            _ => unreachable!(),
        }
    }
}

impl Deref for InflightGuard<'_> {
    type Target = RawTag;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.tag
    }
}

impl Drop for InflightGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        if self.pending {
            let _ = self.tag.abort();
        }
    }
}

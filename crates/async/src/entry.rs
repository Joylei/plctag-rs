// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::*;
use futures_util::{
    future::Future,
    task::{AtomicWaker, Context, Poll},
};
use plctag_core::{ffi::PLCTAG_ERR_NOT_FOUND, Decode, Encode};
use std::{
    ffi::c_void,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering},
};

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
    waker: AtomicWaker,
    state: AtomicU8,
    set: AtomicBool,
    event: AtomicI32,
    status: AtomicI32,
}

impl Inner {
    fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
            state: Default::default(),
            set: AtomicBool::new(false),
            event: AtomicI32::new(0),
            status: AtomicI32::new(0),
        }
    }

    #[inline]
    fn state(&self) -> u8 {
        self.state.load(Ordering::Acquire)
    }

    #[inline]
    fn take_event(&self) -> (i32, i32) {
        let event = self.event.swap(0, Ordering::Relaxed);
        let status = self.status.swap(0, Ordering::Relaxed);
        self.set.store(false, Ordering::Release);
        (event, status)
    }

    #[inline]
    fn set_event(&self, event: i32, status: i32) {
        match event {
            PLCTAG_EVENT_CREATED => {
                //dbg!("TAG_CREATED");
                self.state.store(TAG_CRATED, Ordering::Relaxed);
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
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return;
                }
            }
            PLCTAG_EVENT_DESTROYED => {
                self.state.store(TAG_DESTROYED, Ordering::Relaxed);
            }
            _ => {}
        }
        self.event.store(event, Ordering::Relaxed);
        self.status.store(status, Ordering::Relaxed);
        self.set.store(true, Ordering::Relaxed);
        self.waker.wake();
    }

    fn notified(&self) -> Notified<'_> {
        Notified(&self)
    }
}

struct Notified<'a>(&'a Inner);

impl Future for Notified<'_> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.set.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }
        self.0.waker.register(cx.waker());
        if self.0.set.load(Ordering::Relaxed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug)]
struct ArcGuard<T> {
    ptr: *const T,
}

impl<T> Drop for ArcGuard<T> {
    #[inline]
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
                    inner.set_event(event, status);
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
        let mut state = self.inner.state.load(Ordering::Acquire);
        loop {
            match state {
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
            self.inner.notified().await;
            state = self.inner.state.load(Ordering::Relaxed);
        }
    }

    /// wait until created
    #[inline]
    pub async fn ready(&mut self) -> Result<()> {
        let mut state = self.inner.state.load(Ordering::Acquire);
        loop {
            match state {
                TAG_DESTROYED => {
                    return Err(Status::Err(PLCTAG_ERR_NOT_FOUND).into());
                }
                TAG_CRATED | TAG_FIRST_READ => {
                    return Ok(());
                }
                _ => {}
            }
            self.inner.notified().await;
            state = self.inner.state.load(Ordering::Relaxed);
        }
    }

    /// perform read operation.
    #[inline]
    pub async fn read(&mut self) -> Result<()> {
        self.read_or_write(PLCTAG_EVENT_READ_COMPLETED).await
    }

    /// perform write operation
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
        debug_assert!(!status.is_pending());
        if status.is_err() {
            guard.pending = false;
            return Err(status.into());
        }
        drop(guard);
        Ok(())
    }

    /// poll status
    #[inline]
    pub fn status(&mut self) -> Status {
        match self.inner.state() {
            TAG_DESTROYED => Status::Err(PLCTAG_ERR_NOT_FOUND),
            _ => self.tag.status(),
        }
    }

    /// get tag attribute
    #[inline]
    pub fn get_attr(&mut self, attr: impl AsRef<str>, default_value: i32) -> Result<i32> {
        Ok(self.tag.get_attr(attr, default_value)?)
    }

    /// set tag attribute
    #[inline]
    pub fn set_attr(&mut self, attr: impl AsRef<str>, value: i32) -> Result<()> {
        Ok(self.tag.set_attr(attr, value)?)
    }

    /// element size
    #[inline]
    pub fn elem_size(&mut self) -> Result<i32> {
        Ok(self.tag.elem_size()?)
    }

    /// element count
    #[inline]
    pub fn elem_count(&mut self) -> Result<i32> {
        Ok(self.tag.elem_count()?)
    }

    /// tag size in bytes
    #[inline]
    pub fn size(&mut self) -> Result<u32> {
        Ok(self.tag.size()?)
    }

    /// set tag size in bytes, returns old size
    #[inline]
    pub fn set_size(&mut self, size: u32) -> Result<u32> {
        Ok(self.tag.set_size(size)?)
    }

    /// get bit value
    #[inline]
    pub fn get_bit(&mut self, bit_offset: u32) -> Result<bool> {
        Ok(self.tag.get_bit(bit_offset)?)
    }

    /// set bit value
    #[inline]
    pub fn set_bit(&mut self, bit_offset: u32, value: bool) -> Result<()> {
        Ok(self.tag.set_bit(bit_offset, value)?)
    }

    /// get value from mem, you should call read() before this operation
    #[inline]
    #[cfg(feature = "value")]
    pub fn get_value<T: Decode>(&mut self, byte_offset: u32) -> Result<T> {
        use plctag_core::ValueExt;
        let v = self.tag.get_value(byte_offset)?;
        Ok(v)
    }

    /// set value in mem, you should call write() later
    #[inline]
    #[cfg(feature = "value")]
    pub fn set_value<T: Encode>(&mut self, byte_offset: u32, value: T) -> Result<()> {
        use plctag_core::ValueExt;
        self.tag.set_value(byte_offset, value)?;
        Ok(())
    }

    /// perform read & returns the value
    #[inline]
    #[cfg(feature = "value")]
    pub async fn read_value<T: Decode>(&mut self, offset: u32) -> Result<T> {
        use plctag_core::ValueExt;
        self.read().await?;
        //dbg!("read done", self.tag.status());
        Ok(self.tag.get_value(offset)?)
    }

    /// set the value and write to PLC Controller
    #[cfg(feature = "value")]
    #[inline]
    pub async fn write_value<T: Encode + Send>(&mut self, offset: u32, value: T) -> Result<()> {
        use plctag_core::ValueExt;
        self.ready().await?;
        self.tag.set_value(offset, value)?;
        self.write().await?;
        Ok(())
    }

    /// get raw bytes
    #[inline]
    pub fn get_bytes(&mut self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        let v = self.tag.get_bytes(byte_offset, buf)?;
        Ok(v)
    }

    /// get raw bytes.
    /// If buffer length would exceed the end of the data in the tag data buffer, an out of bounds error is returned
    pub fn get_bytes_unchecked(&self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        Ok(self.tag.get_bytes_unchecked(byte_offset, buf)?)
    }

    /// set raw bytes
    #[inline]
    pub fn set_bytes(&mut self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        Ok(self.tag.set_bytes(byte_offset, buf)?)
    }

    /// set raw bytes.
    /// If buffer length would exceed the end of the data in the tag data buffer, an out of bounds error is returned
    #[inline]
    pub fn set_bytes_unchecked(&mut self, byte_offset: u32, buf: &[u8]) -> Result<usize> {
        Ok(self.tag.set_bytes_unchecked(byte_offset, buf)?)
    }

    /// take the inner
    pub fn into_inner(self) -> RawTag {
        unsafe {
            self.tag.unregister_callback();
        }
        self.tag
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

impl Drop for InflightGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        if self.pending {
            let _ = self.tag.abort();
        }
    }
}

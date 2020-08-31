#![cfg(feature = "async")]

use super::{asyncify, get_status};
use crate::{RawTag, Result, Status, TagValue};
use std::cell::UnsafeCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time;

#[doc(hidden)]
pub(crate) struct Inner {
    raw: RawTag,
    /// event need to be mutable, but `Arc` wrapped `Inner` is not mutable, wrap it with `UnsafeCell`
    event: UnsafeCell<event::Event>,
}

unsafe impl Sync for Inner {}
unsafe impl Send for Inner {}

/// async wrapper of `RawTag`
///
/// # Examples
///
/// ```rust,ignore
/// use plctag::future::AsyncTag;
/// use tokio::runtime::Runtime;
///
/// let mut rt = Runtime::new()::unwrap();
/// rt.block_on(async move {
///     // YOUR TAG DEFINITION
///     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
///     let tag = AsyncTag::new(path).await.unwrap();
///     
///     let offset = 0;
///     let value:u16 = 100;
///     //write tag
///     tag.set_and_write(offset, value).await.unwrap();
///     // read tag
///     let value:u16 = tag.read_and_get(offset).await.unwrap();
///     assert_eq!(value, 100);
/// });
///
/// ```
pub struct AsyncTag {
    inner: Arc<Inner>,
}

impl AsyncTag {
    /// create new instance of `AsyncTag`
    /// # Note
    /// if you passed wrong path parameters, your program might crash.
    /// you'd better use `PathBuilder` to build a path.
    pub async fn new(path: impl AsRef<str>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let raw = asyncify(move || RawTag::new(path, 0)).await?;
        let inner = Arc::new(Inner {
            raw,
            event: UnsafeCell::new(event::Event::new()),
        });
        // no efficient way to know when tag will be created, constantly poll status
        loop {
            let inner2 = Arc::clone(&inner);
            let status = get_status(move || inner2.raw.status()).await;

            if status.is_ok() {
                break;
            }
            if status.is_err() {
                return Err(status.into());
            }
            time::delay_for(Duration::from_millis(1)).await
        }
        event::register(Arc::clone(&inner));
        Ok(Self { inner })
    }

    /// tag id in `libplctag`.
    ///
    /// # Note
    ///
    /// The id is not a resource handle.
    /// The id might be reused by `libplctag`. So if you use it somewhere, please take care.
    pub fn id(&self) -> i32 {
        self.inner.raw.id()
    }

    /// poll tag status
    pub async fn status(&self) -> Status {
        let inner = Arc::clone(&self.inner);
        let status = get_status(move || inner.raw.status()).await;
        status
    }

    /// value size of bytes
    pub async fn size(&self) -> Result<u32> {
        let inner = Arc::clone(&self.inner);
        asyncify(move || inner.raw.size()).await
    }

    /// element size
    pub async fn element_size(&self) -> Result<i32> {
        self.get_attr("elem_size", 0).await
    }

    /// element count
    pub async fn element_count(&self) -> Result<i32> {
        self.get_attr("elem_count", 0).await
    }

    pub async fn get_attr(&self, attr: impl AsRef<str>, default_value: i32) -> Result<i32> {
        let inner = Arc::clone(&self.inner);
        let attr = attr.as_ref().to_owned();
        asyncify(move || inner.raw.get_attr(attr, default_value)).await
    }

    pub async fn set_attr(&self, attr: impl AsRef<str>, value: i32) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let attr = attr.as_ref().to_owned();
        asyncify(move || inner.raw.set_attr(attr, value)).await
    }

    pub async fn read_and_get<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        let status = self.read().await;
        if !status.is_ok() {
            return Err(status.into());
        }
        self.get_value(offset).await
    }
    pub async fn set_and_write<T: TagValue + Send + 'static>(
        &self,
        offset: u32,
        value: T,
    ) -> Result<()> {
        self.set_value(offset, value).await?;
        let status = self.write().await;
        if status.is_ok() {
            Ok(())
        } else {
            Err(status.into())
        }
    }

    /// read the value from plc
    pub async fn read(&self) -> Status {
        let inner = Arc::clone(&self.inner);
        let status = get_status(move || inner.raw.read(0)).await;
        if !status.is_pending() {
            return status; // either ok or err
        }
        let inner = Arc::clone(&self.inner);
        let waiter = Waiter::new(inner);
        let res: event::EventArgs = waiter.await;
        debug_assert!(!res.is_default());
        res.status()
    }

    /// write the value to plc
    pub async fn write(&self) -> Status {
        let inner = Arc::clone(&self.inner);
        let status = get_status(move || inner.raw.write(0)).await;
        if !status.is_pending() {
            return status; // either ok or err
        }
        let inner = Arc::clone(&self.inner);
        let waiter = Waiter::new(inner);
        let res: event::EventArgs = waiter.await;
        res.status()
    }

    /// get value from tag
    pub async fn get_value<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        let inner = Arc::clone(&self.inner);
        asyncify(move || {
            let mut v: T = Default::default();
            v.get_value(&inner.raw, offset)?;
            Ok(v)
        })
        .await
    }

    /// set value for the tag
    pub async fn set_value<T: TagValue + Send + 'static>(
        &self,
        offset: u32,
        value: T,
    ) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        asyncify(move || value.set_value(&inner.raw, offset)).await
    }

    pub async fn get_bytes(&self, buf: &'static mut [u8]) -> Result<usize> {
        let inner = Arc::clone(&self.inner);
        asyncify(move || inner.raw.get_bytes(buf)).await
    }

    pub async fn set_bytes(&self, buf: &'static [u8]) -> Result<usize> {
        let inner = Arc::clone(&self.inner);
        asyncify(move || inner.raw.set_bytes(buf)).await
    }
}

impl Drop for AsyncTag {
    fn drop(&mut self) {
        event::unregister(self.inner.raw.id());
    }
}

/// wait for read/write event call back.
/// automatically abort the pending operation when dropped
#[doc(hidden)]
struct Waiter {
    inner: Arc<Inner>,
}

impl Waiter {
    #[inline]
    pub fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }
}

impl Future for Waiter {
    type Output = event::EventArgs;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let evt: Pin<&mut event::Event> =
            unsafe { Pin::new_unchecked(&mut *self.inner.event.get()) };
        evt.poll(ctx)
    }
}

impl Drop for Waiter {
    fn drop(&mut self) {
        let event = unsafe { &mut *self.inner.event.get() };
        if !event.ready() {
            let _ = self.inner.raw.abort();
        }
        event.reset();
    }
}

#[doc(hidden)]
mod event {
    use super::Inner;
    use crate::Status;
    use futures::task::AtomicWaker;
    use parking_lot;
    use std::collections::HashMap;
    use std::future::Future;
    use std::ops::Deref;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll};

    lazy_static! {
        static ref TAGS: parking_lot::Mutex<HashMap<i32, TagHolder>> =
            parking_lot::Mutex::new(HashMap::new());
    }

    /// `Arc` is not allowed to be put into HashMap in `lazy_static`, so wrap it with `TagHolder`
    #[doc(hidden)]
    pub(crate) struct TagHolder(Arc<Inner>);

    impl Clone for TagHolder {
        #[inline]
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }

    impl Deref for TagHolder {
        type Target = Inner;
        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    unsafe impl Send for TagHolder {}
    unsafe impl Sync for TagHolder {}

    #[inline]
    fn get_holder(tag_id: i32) -> Option<TagHolder> {
        let data = TAGS.lock();
        let map = &*data;
        if let Some(holder) = map.get(&tag_id) {
            Some(holder.clone())
        } else {
            None
        }
    }

    const EVENT_READ_DONE: i32 = 2;
    const EVENT_WRITE_DONE: i32 = 4;

    extern "C" fn on_tag_event(tag_id: i32, event: i32, status: i32) {
        if let Some(ref holder) = get_holder(tag_id) {
            if event == EVENT_READ_DONE || event == EVENT_WRITE_DONE {
                let args = EventArgs { event, status };
                let evt = unsafe { &*holder.event.get() };
                evt.fire(args);
            }
        }
    }

    /// register tag, so it can be notified when event received
    #[inline]
    pub(crate) fn register(inner: Arc<Inner>) {
        let mut data = TAGS.lock();
        let map = &mut *data;
        let status = unsafe { inner.raw.register_callback(Some(on_tag_event)) };
        debug_assert!(status.is_ok());
        map.insert(inner.raw.id(), TagHolder(inner));
    }

    /// unregister tag,
    #[inline]
    pub fn unregister(tag_id: i32) {
        let mut data = TAGS.lock();
        let map = &mut *data;
        if let Some(holder) = map.remove(&tag_id) {
            holder.raw.unregister_callback();
        }
    }

    #[inline]
    pub fn is_registered(tag_id: i32) -> bool {
        let data = TAGS.lock();
        let map = &*data;
        map.contains_key(&tag_id)
    }

    /// event callback data
    #[doc(hidden)]
    #[derive(Debug, Clone)]
    pub(crate) struct EventArgs {
        event: i32,
        status: i32,
    }

    impl EventArgs {
        #[inline]
        pub fn is_default(&self) -> bool {
            self.event == i32::MIN
        }

        #[inline]
        pub fn status(&self) -> Status {
            Status::new(self.status)
        }
    }

    impl Default for EventArgs {
        #[inline]
        fn default() -> Self {
            Self {
                event: i32::MIN,
                status: i32::MIN,
            }
        }
    }

    #[doc(hidden)]
    pub(crate) struct Event {
        waker: AtomicWaker,
        args: parking_lot::Mutex<EventArgs>,
        ready: AtomicBool,
    }

    impl Event {
        pub fn new() -> Self {
            Self {
                waker: AtomicWaker::new(),
                args: parking_lot::Mutex::new(Default::default()),
                ready: AtomicBool::new(false),
            }
        }

        #[inline]
        pub fn ready(&self) -> bool {
            let args = &mut *self.args.lock();
            !args.is_default()
        }

        fn fire(&self, args: EventArgs) {
            let args_inner = &mut *self.args.lock();
            *args_inner = args;
            self.ready.store(true, Ordering::Relaxed);
            self.waker.wake();
        }

        #[inline]
        fn args(&self) -> EventArgs {
            let args = &mut *self.args.lock();
            args.clone()
        }

        #[inline]
        pub fn reset(&self) {
            let args = &mut *self.args.lock();
            if !args.is_default() {
                *args = Default::default();
            }
            self.ready.store(false, Ordering::Relaxed);
        }
    }

    impl Future for Event {
        type Output = EventArgs;

        fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready.load(Ordering::Relaxed) {
                return Poll::Ready(self.args());
            }
            self.waker.register(ctx.waker());
            if self.ready.load(Ordering::Relaxed) {
                return Poll::Ready(self.args());
            }
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_async_tag() {
        let task = async move {
            let path = "make=system&family=library&name=debug&debug=4";
            let res = AsyncTag::new(path.to_owned()).await;
            assert!(res.is_ok());
            let tag = res.unwrap();

            // let size = tag.size().await.unwrap();
            // assert!(size > 0);
            // let res = tag.read_and_get(0).await;
            // assert!(res.is_ok());
            // let level: u32 = res.unwrap();
            // assert_eq!(level, 4);

            let res = tag.set_and_write(0, 1 as u32).await;
            assert!(res.is_ok());

            let res = tag.read_and_get(0).await;
            assert!(res.is_ok());
            let level: u32 = res.unwrap();
            assert_eq!(level, 1);

            tag.id()
        };
        let mut rt = Runtime::new().unwrap();
        let tag_id = rt.block_on(task);
        assert!(tag_id > 0);
        assert!(!event::is_registered(tag_id));
    }
}

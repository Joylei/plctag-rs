use crate::ffi;
use crate::options::*;
use crate::{Accessor, RawTag, Result, Status, TagId, TagValue};
use std::cmp::{Eq, PartialEq};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use futures::prelude::*;
use std::future::Future;
use tokio::prelude::*;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time;

pub struct AsyncTag<T: TagValue> {
    tag: Tag<T>,
}

impl<T: TagValue> AsyncTag<T> {
    pub fn new(tag: Tag<T>) -> Self {
        Self { tag }
    }
}

impl<T: TagValue> From<Tag<T>> for AsyncTag<T> {
    fn from(tag: Tag<T>) -> AsyncTag<T> {
        AsyncTag::new(tag)
    }
}

async fn wait(tag: &RawTag) -> Status {
    loop {
        let status = tag.status();
        if !status.is_pending() {
            return status;
        }
        //is pending
        time::delay_for(Duration::from_millis(1)).await;
    }
}

impl<T: TagValue> AsyncTag<T> {
    /// poll tag status
    #[inline]
    pub async fn status(&self) -> Status {
        self.tag.status()
    }

    /// get cached value, please call read() at least once first
    #[inline]
    pub async fn get(&self) -> Result<T> {
        self.tag.get()
    }

    /// ensure underline tag instance created
    #[inline]
    pub async fn create(&mut self, duration: Duration) -> Result<()> {
        if let Some(ref tag) = self.tag.raw {
            let status = tag.status();
            //tag should not pending here
            if status.is_pending() {
                panic!("bad state, tag should not be pending");
            }
            return status.as_result();
        }
        let raw = RawTag::new(&self.tag.path, 0)?;
        let res = time::timeout(duration, wait(&raw)).await;
        if res.is_err() {
            //timeout
            return Err(Status::err_timeout());
        }
        self.tag.raw = Some(Proxy::new(raw));
        Ok(())
    }

    /// read tag, please call `create()` before read
    pub async fn read(&self, duration: Duration) -> Result<T> {
        if let Some(ref tag) = self.tag.raw {
            let status = tag.status();
            //tag should not pending here
            if status.is_pending() {
                panic!("bad state, tag should not be pending");
            }
            let status = tag.raw.read(0);
            if status.is_err() {
                return Err(status);
            }
            let res = time::timeout(duration, wait(&tag.raw)).await;
            if res.is_err() {
                //timeout
                let _ = tag.raw.abort();
                return Err(Status::err_timeout());
            }
            let mut v: T = Default::default();
            v.get_value(tag, 0)?;
            return Ok(v);
        }
        Err(Status::err_create())
    }

    /// write tag, please call `create()` before write
    pub async fn write(&self, value: T, duration: Duration) -> Result<()> {
        if let Some(ref tag) = self.tag.raw {
            let status = tag.status();
            //tag should not pending here
            if status.is_pending() {
                panic!("bad state, tag should not be pending");
            }
            value.set_value(tag, 0)?;
            // will pending
            let status = tag.raw.write(0);
            if status.is_err() {
                return Err(status);
            }
            let res = time::timeout(duration, wait(&tag.raw)).await;
            if res.is_err() {
                //timeout, abort pending
                let _ = tag.raw.abort();
                return Err(Status::err_timeout());
            }
            return Ok(());
        }
        Err(Status::err_create())
    }
}

/// typed plc tag, wrapper on top of `RawTag`
pub struct Tag<T: TagValue> {
    /// unique id
    id: String,
    /// tag construct path
    path: String,
    raw: Option<Proxy>,
    _marker: PhantomData<T>,
}

impl<T: TagValue> Tag<T> {
    pub(crate) fn create(controller_opts: &ControllerOptions, tag_opts: &TagOptions) -> Self {
        let path = build_path(controller_opts, tag_opts);
        let id = generate_id(controller_opts, tag_opts);
        Tag::new(&id, &path)
    }

    pub fn new(id: &str, path: &str) -> Self {
        Self {
            id: id.to_string(),
            path: path.to_string(),
            raw: None,
            _marker: PhantomData,
        }
    }

    /// read tag
    #[inline]
    pub fn read(&mut self, timeout: Duration) -> Result<()> {
        let timeout_ms = timeout.as_millis() as u32;
        if let Some(ref tag) = self.raw {
            let status = tag.raw.read(timeout_ms);
            if status.is_ok() {
                return Ok(());
            }
            return Err(status);
        }
        Err(Status::err_create())
    }

    /// read and get the value
    pub fn read_and_get(&mut self, timeout: Duration) -> Result<T> {
        if let Err(mut status) = self.read(timeout) {
            while status.is_pending() {
                std::thread::sleep(Duration::from_millis(1));
                status = self.status();
            }
            if status.is_err() {
                return Err(status);
            }
        }
        self.get()
    }

    /// write tag
    #[inline]
    pub fn write(&mut self, timeout: Duration) -> Result<()> {
        let timeout_ms = timeout.as_millis() as u32;
        if let Some(ref tag) = self.raw {
            //normally we should not wait, in case in bad state
            let status = tag.raw.write(timeout_ms);
            if status.is_ok() {
                return Ok(());
            }
            return Err(status);
        }
        Err(Status::err_create())
    }

    pub fn set_and_write(&mut self, value: T, timeout: Duration) -> Result<()> {
        self.set(value)?;
        self.write(timeout)
    }

    /// read value
    #[inline]
    pub fn get(&self) -> Result<T> {
        if let Some(ref tag) = self.raw {
            let mut value: T = Default::default();
            value.get_value(tag, 0)?; //no pending
            return Ok(value);
        }
        Err(Status::err_create())
    }

    /// set value
    #[inline]
    pub fn set(&self, value: T) -> Result<()> {
        if let Some(ref tag) = self.raw {
            return value.set_value(tag, 0);
        }
        Err(Status::err_create())
    }
}

impl<T: TagValue> ITag for Tag<T> {
    /// unique identifier for the tag
    #[inline]
    fn id(&self) -> &str {
        &self.id
    }

    ///create the underline tag if  not created
    #[inline]
    fn create(&mut self, timeout: Duration) -> Result<()> {
        let timeout = timeout.as_millis() as u32;
        if let Some(ref _tag) = self.raw {
            Ok(())
        } else {
            let res = RawTag::new(&self.path, timeout).and_then(|tag| Ok(Some(Proxy::new(tag))));
            match res {
                Ok(tag) => {
                    self.raw = tag;
                    return Ok(());
                }
                Err(status) => Err(status),
            }
        }
    }

    /// reset the raw tag
    #[inline]
    fn reset(&mut self) {
        self.raw = None;
    }

    /// poll status
    #[inline]
    fn status(&self) -> Status {
        if let Some(ref tag) = self.raw {
            tag.status()
        } else {
            Status::err_create()
        }
    }
}

impl<T: TagValue> Hash for Tag<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl<T: TagValue> PartialEq for Tag<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id().eq(other.id())
    }
}

impl<T: TagValue> Eq for Tag<T> {}

/// with `Proxy`, only expose minimal APIs for operations on `TagValue`
pub struct Proxy {
    raw: RawTag,
}

impl TagId for Proxy {
    #[inline]
    fn id(&self) -> i32 {
        self.raw.id()
    }
}

impl Accessor for Proxy {}
impl Proxy {
    #[inline]
    pub(crate) fn new(raw: RawTag) -> Self {
        Self { raw }
    }
}

/// abstract tag interface
pub trait ITag {
    /// unique identifier for the tag
    fn id(&self) -> &str;

    /// ensure raw tag created
    fn create(&mut self, timeout: Duration) -> Result<()>;

    /// reset: destroy tag internally, can be re-created as needed
    fn reset(&mut self);

    fn status(&self) -> Status;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plc::shutdown;

    #[test]
    fn test_tag() {
        let timeout = Duration::from_millis(100);
        let path = "make=system&family=library&name=debug&debug=4";
        let mut tag: Tag<u32> = Tag::new("debug", path);
        let res = tag.create(timeout);
        assert!(res.is_ok());
        let res = tag.read_and_get(timeout);
        assert_eq!(res.unwrap(), 4);

        let res = tag.set_and_write(1, timeout);
        assert!(res.is_ok());

        let res = tag.read_and_get(timeout);
        assert_eq!(res.unwrap(), 1);
    }

    #[test]
    fn test_async() {
        use tokio::runtime::Runtime;

        let mut rt = Runtime::new().unwrap();

        let path = "make=system&family=library&name=debug&debug=4";

        let duration = Duration::from_millis(100);
        rt.block_on(async move {
            let mut tag: AsyncTag<u32> = Tag::new("debug", path).into();
            let res = tag.create(duration).await;
            assert!(res.is_ok());

            let res = tag.read(duration).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), 4);

            let res = tag.write(1, duration).await;
            assert!(res.is_ok());
            let res = tag.read(duration).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), 1);
        });
    }
}

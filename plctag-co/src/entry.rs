use std::time::{Duration, Instant};

use crate::*;
use may::coroutine as co;
use plctag::RawTag;

pub struct TagEntry {
    inner: Arc<Inner>,
}

struct Inner {
    tag: RawTag,
    lock: may::sync::Mutex<()>,
}

impl Clone for TagEntry {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl TagEntry {
    pub fn create(options: impl Into<String>, timeout: Option<Duration>) -> Result<Self> {
        let path = options.into();
        let tag = RawTag::new(path.clone(), 0)?;
        let is_timeout = if let Some(timeout) = timeout {
            let expire_at = Instant::now() + timeout;
            Some(move || expire_at < Instant::now())
        } else {
            None
        };
        loop {
            let status = tag.status();
            if status.is_pending() {
                //check timeout
                if is_timeout.map(|f| f()).unwrap_or(false) {
                    return Err(Error::Timeout);
                }
                //co:: yield_now();
                co::sleep(Duration::from_millis(1));
                continue;
            }
            status.into_result()?;
            //is ok
            break;
        }
        Ok(Self {
            inner: Arc::new(Inner {
                tag,
                lock: may::sync::Mutex::new(()),
            }),
        })
    }

    pub fn get(&self) -> Result<TagRef<'_, RawTag>> {
        let lock = self.inner.lock.lock()?;
        let tag = &self.inner.tag;
        Ok(TagRef { tag, lock })
    }
}

impl From<RawTag> for TagEntry {
    #[inline(always)]
    fn from(tag: RawTag) -> Self {
        Self {
            inner: Arc::new(Inner {
                tag,
                lock: may::sync::Mutex::new(()),
            }),
        }
    }
}

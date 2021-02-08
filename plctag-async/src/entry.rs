use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicUsize},
    task::{Context, Poll},
};

use futures::channel::oneshot;
use mailbox::Token;
use plctag::{event::Remover, Accessor};
use plctag_sys as ffi;
use tokio::sync::Notify;

use crate::*;

#[inline(always)]
async fn asyncify<F, R>(f: F) -> Result<R>
where
    F: FnOnce() -> std::result::Result<R, Status> + Send + 'static,
    R: Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(Error::TagError(e)),
        Err(e) => Err(Error::TaskError(e)),
    }
}

pub struct TagEntry<O: TagOptions> {
    opts: O,
    token: Arc<Token>,
}

impl<O: TagOptions> TagEntry<O> {
    #[inline(always)]
    pub(crate) fn new(opts: O, token: Token) -> Self {
        Self {
            opts,
            token: Arc::new(token),
        }
    }

    #[inline(always)]
    pub async fn wait_ready(&self) {
        self.token.wait().await
    }

    #[inline(always)]
    pub fn key(&self) -> &str {
        self.opts.key()
    }

    #[inline(always)]
    async fn with_tag<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&RawTag) -> plctag::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let token = Arc::clone(&self.token);
        asyncify(move || {
            let tag = token.get()?;
            f(tag)
        })
        .await
    }

    #[inline(always)]
    pub async fn size(&self) -> Result<u32> {
        self.with_tag(|tag| tag.size()).await
    }

    #[inline(always)]
    pub async fn elem_size(&self) -> Result<i32> {
        self.with_tag(|tag| tag.elem_size()).await
    }

    #[inline(always)]
    pub async fn elem_count(&self) -> Result<i32> {
        self.with_tag(|tag| tag.elem_count()).await
    }

    /// read from plc to memory
    #[inline(always)]
    pub async fn read(&self) -> Result<()> {
        let mut op = Operation::new(&self.token);
        op.run(true).await
    }

    /// write to plc
    #[inline(always)]
    pub async fn write(&self) -> Result<()> {
        let mut op = Operation::new(&self.token);
        op.run(false).await
    }

    /// read from plc and returns the value
    #[inline(always)]
    pub async fn read_value<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        self.read().await?;
        self.get_value(offset).await
    }
    /// write the specified value to plc

    #[inline(always)]
    pub async fn write_value(
        &self,
        offset: u32,
        value: impl TagValue + Send + 'static,
    ) -> Result<()> {
        self.set_value(offset, value).await?;
        self.write().await
    }

    /// get value in memory
    #[inline(always)]
    pub async fn get_value<T: TagValue + Send + 'static>(&self, offset: u32) -> Result<T> {
        self.with_tag(move |tag| tag.get_value(offset)).await
    }

    /// set value  in memory
    #[inline(always)]
    pub async fn set_value(
        &self,
        offset: u32,
        value: impl TagValue + Send + 'static,
    ) -> Result<()> {
        self.with_tag(move |tag| tag.set_value(offset, value)).await
    }
}

/// ensures that pending operation get aborted if not successful
struct Operation<'a> {
    /// should abort or not
    status: Status,
    token: &'a Arc<Token>,
}

impl<'a> Operation<'a> {
    #[inline(always)]
    fn new(token: &'a Arc<Token>) -> Self {
        Self {
            status: Status::Ok,
            token,
        }
    }

    async fn run(&mut self, rw: bool) -> Result<()> {
        let tag = self.token.get()?;
        self.status = Status::Pending;
        let (tx, rx) = oneshot::channel();
        //Option hack for moving out value
        let mut tx = Some(tx);
        let removal = tag
            .listen(move |evt, status| {
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
                tx.take().map(|tx| {
                    let _ = tx.send(status);
                });
            })
            .manual(false)
            .on();
        let status = {
            let token = Arc::clone(&self.token);

            task::spawn_blocking(move || {
                let tag = token.get().unwrap();

                if rw {
                    tag.read(0)
                } else {
                    tag.write(0)
                }
            })
            .await?
        };
        self.status = status;
        if status.is_err() {
            status.into_result()?;
        } else if status.is_pending() {
            self.status = rx.await.map_err(|e| Error::RecvError)?;
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

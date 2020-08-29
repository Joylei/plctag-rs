#![cfg(feature = "async")]

mod tag;

use crate::{Result, Status};
use tokio::task;

pub use tag::AsyncTag;

#[inline]
pub(crate) async fn asyncify<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(Status::err_task()),
    }
}

#[inline]
pub(crate) async fn asyncify2<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(res) => Ok(res),
        Err(_) => Err(Status::err_task()),
    }
}

#[inline]
pub(crate) async fn get_status<F>(f: F) -> Status
where
    F: FnOnce() -> Status + Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Status::err_task(),
    }
}

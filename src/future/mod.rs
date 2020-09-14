//! async operations

#![cfg(feature = "async")]

#[cfg(feature = "controller")]
pub mod controller;
mod tag;

use crate::{Result, Status};
use tokio::task;

pub use tag::{AsyncTag, Buf};

pub mod prelude {
    #[cfg(feature = "controller")]
    pub use super::controller::*;
    pub use super::tag::{AsyncTag, Buf};
    pub use crate::{Result, Status};
}

#[inline]
pub(crate) async fn asyncify<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(Status::err_task().into()),
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

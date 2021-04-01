use std::{sync::Arc, time::Duration};

use crate::Result;
use may::sync::Blocker;
use once_cell::sync::OnceCell;
use plctag::Status;

pub struct SyncCell {
    cell: Arc<OnceCell<Status>>,
    blocker: Arc<Blocker>,
}

impl SyncCell {
    #[inline(always)]
    pub fn new() -> Self {
        let blocker = Blocker::current();
        let cell = Arc::new(OnceCell::new());
        Self { cell, blocker }
    }
    #[inline(always)]
    pub fn wait(&self, timeout: Option<Duration>) -> Result<Status> {
        //try get value first
        if let Some(v) = self.cell.get() {
            return Ok(*v);
        }
        self.blocker.park(timeout)?;
        let v = self.cell.get().unwrap();
        Ok(*v)
    }
    #[inline(always)]
    pub fn set(&self, val: Status) {
        if self.cell.set(val).is_ok() {
            self.blocker.unpark();
        }
    }
}

impl Clone for SyncCell {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            cell: Arc::clone(&self.cell),
            blocker: Arc::clone(&self.blocker),
        }
    }
}

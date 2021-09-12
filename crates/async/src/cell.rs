// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use std::{
    cell::UnsafeCell,
    hint,
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicU8, Ordering},
};
use tokio::sync::Notify;

const STATUS_NOTIFY_ALL: u8 = 1;
const STATUS_LOCKED: u8 = 2;
const STATUS_VALUE_SET: u8 = 4;

/// async OnceCell
#[derive(Debug)]
pub struct OnceCell<T> {
    cell: UnsafeCell<MaybeUninit<T>>,
    status: AtomicU8,
    flag: Notify,
}

impl<T> OnceCell<T> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::new_internal(false)
    }

    #[inline(always)]
    pub fn new_notify_all() -> Self {
        Self::new_internal(true)
    }

    #[inline(always)]
    fn new_internal(all: bool) -> Self {
        Self {
            cell: UnsafeCell::new(MaybeUninit::uninit()),
            status: AtomicU8::new(if all { STATUS_NOTIFY_ALL } else { 0 }),
            flag: Notify::new(),
        }
    }

    /// set value and returns OK if empty, otherwise returns the Value;
    /// notify all waiters
    #[inline(always)]
    pub fn set(&self, val: T) -> std::result::Result<(), T> {
        let mut cur = self.status.load(Ordering::Acquire);
        loop {
            if cur == 0 || cur == STATUS_NOTIFY_ALL {
                let dest = cur | STATUS_LOCKED;
                let res =
                    self.status
                        .compare_exchange(cur, dest, Ordering::AcqRel, Ordering::Acquire);
                match res {
                    Ok(v) => {
                        //lock taken
                        unsafe {
                            let holder = &mut *self.cell.get();
                            holder.as_mut_ptr().write(val);
                        }
                        self.status.store(v | STATUS_VALUE_SET, Ordering::Release);
                        if v & STATUS_NOTIFY_ALL == STATUS_NOTIFY_ALL {
                            self.flag.notify_waiters();
                        } else {
                            self.flag.notify_one();
                        }
                        return Ok(());
                    }
                    Err(v) => {
                        cur = v;
                    }
                }
            }
            // value has been set
            if cur & STATUS_VALUE_SET == STATUS_VALUE_SET {
                return Err(val);
            }
            // locked by another thread
            if cur & STATUS_LOCKED == STATUS_LOCKED {
                hint::spin_loop();
                continue;
            }
            unreachable!();
        }
    }

    #[allow(unused)]
    pub fn is_set(&self) -> bool {
        let status = self.status.load(Ordering::Acquire);
        status & STATUS_VALUE_SET == STATUS_VALUE_SET
    }

    #[allow(unused)]
    fn get_unchecked(&self) -> Option<&T> {
        let status = self.status.load(Ordering::Relaxed);
        if status & STATUS_VALUE_SET == STATUS_VALUE_SET {
            Some(unsafe { &*(*self.cell.get()).as_ptr() })
        } else {
            None
        }
    }

    pub fn get(&self) -> Option<&T> {
        let status = self.status.load(Ordering::Acquire);
        if status & STATUS_VALUE_SET == STATUS_VALUE_SET {
            Some(unsafe { &*(*self.cell.get()).as_ptr() })
        } else {
            None
        }
    }

    #[inline(always)]
    pub async fn wait(&self) -> &T {
        if let Some(v) = self.get() {
            return v;
        }
        self.flag.notified().await;
        if let Some(v) = self.get() {
            v
        } else {
            unreachable!();
        }
    }
}

impl<T> Drop for OnceCell<T> {
    fn drop(&mut self) {
        let status = self.status.load(Ordering::Acquire);
        if status & STATUS_VALUE_SET == STATUS_VALUE_SET {
            unsafe {
                let holder = &mut *self.cell.get();
                ptr::drop_in_place(holder.as_mut_ptr());
            }
        }
    }
}

unsafe impl<T> Send for OnceCell<T> {}
unsafe impl<T> Sync for OnceCell<T> {}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;
    use tokio::task;

    #[test]
    fn test_cell() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let cell = Arc::new(OnceCell::new());
            let flag = Arc::new(Notify::new());
            {
                let cell = Arc::clone(&cell);
                let flag = Arc::clone(&flag);
                task::spawn(async move {
                    let _ = cell.set(1);
                    let _ = cell.set(2);
                    flag.notify_one();
                })
            };
            flag.notified().await;
            let v: &i32 = cell.wait().await;
            assert_eq!(v, &1);
        });
        Ok(())
    }
}

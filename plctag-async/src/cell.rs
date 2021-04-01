use std::cell::UnsafeCell;

use parking_lot::Once;
use tokio::sync::Notify;

pub struct OnceCell<T> {
    once: Once,
    flag: Notify,
    cell: UnsafeCell<Option<T>>,
}

impl<T> OnceCell<T> {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            once: Once::new(),
            flag: Notify::new(),
            cell: UnsafeCell::new(None),
        }
    }
    #[inline(always)]
    pub fn set(&self, val: T) {
        self.once.call_once(|| {
            unsafe {
                *self.cell.get() = Some(val);
            }
            self.flag.notify_one();
        })
    }
    #[inline(always)]
    pub async fn take(&self) -> T {
        self.flag.notified().await;
        let holder = unsafe { &mut *self.cell.get() };
        holder.take().expect("OnceCell: cannot be None here")
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
            let t = {
                let cell = Arc::clone(&cell);
                let flag = Arc::clone(&flag);
                task::spawn(async move {
                    cell.set(1);
                    cell.set(2);
                    flag.notify_one();
                })
            };
            flag.notified().await;
            let v: i32 = cell.take().await;
            assert_eq!(v, 1);
        });
        Ok(())
    }
}

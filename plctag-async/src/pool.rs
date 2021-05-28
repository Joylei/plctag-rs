use crate::{cell::OnceCell, private::TagRef, Result};

use plctag::{RawTag, Status};

use parking_lot::Mutex;
use std::{
    collections::{BTreeSet, HashMap},
    future::Future,
    ops::Deref,
    sync::Arc,
};

use tokio::{
    sync::{mpsc, Notify},
    task,
    time::{self, Duration, Instant},
};

#[doc(hidden)]
pub trait Initialize: Send + Sync {
    fn create(path: String) -> plctag::Result<Self>
    where
        Self: Sized;
    fn status(&self) -> Status;
}

impl Initialize for RawTag {
    #[inline(always)]
    fn create(path: String) -> plctag::Result<Self> {
        let tag = RawTag::new(path, 0)?;
        Ok(tag)
    }

    #[inline(always)]
    fn status(&self) -> Status {
        RawTag::status(self)
    }
}

#[derive(Debug)]
pub struct PoolOptions {
    expire_after: Duration,
    fault_last: Duration,
}

impl PoolOptions {
    #[inline]
    pub fn new() -> Self {
        Self {
            expire_after: Duration::from_secs(60),
            fault_last: Duration::from_secs(1),
        }
    }

    /// default for 60 seconds
    #[inline]
    pub fn expire_after(mut self, period: Duration) -> Self {
        self.expire_after = period;
        self
    }

    #[inline]
    pub fn fault_last(mut self, period: Duration) -> Self {
        self.fault_last = period;
        self
    }

    #[inline]
    pub fn create<T: Initialize + 'static>(self) -> Pool<T> {
        Pool::new_with_options(self)
    }
}

impl Default for PoolOptions {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Tag Pool
#[derive(Debug)]
pub struct Pool<T> {
    shared: Arc<Wrapper<T>>,
}

impl<T: Initialize> Clone for Pool<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: Initialize + 'static> Pool<T> {
    #[inline]
    pub fn new() -> Self {
        Self::new_with_options(Default::default())
    }

    #[inline]
    fn new_with_options(options: PoolOptions) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let scan_task = ScanTask::new(tx);
        let shared = Arc::new(Shared::new(options, scan_task));
        let wrapper = Arc::new(Wrapper(shared));
        task::spawn(purge_expired_tags_task(Arc::clone(&wrapper)));
        task::spawn(scan_tags_task(rx, Arc::clone(&wrapper)));

        Self { shared: wrapper }
    }

    /// get or create tag, returns after created
    #[inline]
    pub async fn entry(&self, tag_path: impl AsRef<str>) -> Result<Entry<T>> {
        let entry = {
            let state = &mut *self.shared.state.lock();
            if let Some(entry) = state.will_alive(tag_path.as_ref()) {
                entry.clone()
            } else {
                // create if not exist
                let res = state.add_entry(tag_path.as_ref().to_owned())?;
                //add for scanning
                self.shared.scan_task.add(Arc::clone(&res.state));
                res
            }
        };

        Ok(Entry {
            inner: entry,
            shared: Arc::clone(&self.shared.0),
        })
    }

    /// remove entry from pool
    #[inline]
    pub fn remove(&self, tag_path: impl AsRef<str>) -> Option<Entry<T>> {
        let mut state = self.shared.state.lock();
        state.remove_entry(tag_path.as_ref()).map(|item| {
            self.shared.scan_task.remove(item.id());
            Entry {
                inner: item.inner,
                shared: Arc::clone(&self.shared.0),
            }
        })
    }

    #[inline(always)]
    pub fn contains(&self, tag_path: impl AsRef<str>) -> bool {
        let state = self.shared.state.lock();
        state.tag_by_path(tag_path.as_ref()).is_some()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        let state = self.shared.state.lock();
        state.entries.len()
    }

    pub async fn for_each<F, Fut>(&self, f: F) -> Result<()>
    where
        F: Fn(Entry<T>) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let state = self.shared.state.lock();
        let entries: Vec<_> = state
            .entries
            .values()
            .map(|item| item.inner.clone())
            .collect();
        drop(state);
        for item in entries {
            let entry = Entry {
                inner: item,
                shared: Arc::clone(&self.shared.0),
            };
            f(entry).await?;
        }
        Ok(())
    }
}

impl<T> Drop for Pool<T> {
    fn drop(&mut self) {
        // - this ref
        // - purge task
        // - scan task
        if Arc::strong_count(&self.shared) == 3 {
            debug!("drop pool");
            let mut state = self.shared.state.lock();
            state.shutdown = true;
            drop(state);
            // send signals to tasks
            self.shared.purge_task.notify_one();
            self.shared.scan_task.flag.notify_one();
        }
    }
}

impl<T: Initialize + 'static> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct Wrapper<T>(Arc<Shared<T>>);

impl<T> Deref for Wrapper<T> {
    type Target = Shared<T>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Tag Entry State
#[derive(Debug)]
struct EntryState<T> {
    /// unique id for this instance
    id: u64,
    /// tag options
    path: String,
    tag: T,
    err_status: OnceCell<Status>,
}

unsafe impl<T> Send for EntryState<T> {}
unsafe impl<T> Sync for EntryState<T> {}

#[derive(Debug)]
pub struct Entry<T: Initialize> {
    inner: EntryInner<T>,
    shared: Arc<Shared<T>>,
}

impl<T: Initialize> Drop for Entry<T> {
    fn drop(&mut self) {
        //dbg!(Arc::strong_count(&self.inner.lock));
        let should_expire = Arc::strong_count(&self.inner.lock) == 2;
        if should_expire {
            debug!("PoolState: entry[{}] will expire", self.inner.state.id);
            //already set expiration for error state, skip
            self.shared.will_expire(self.inner.state.id, false, None);
        }
    }
}

impl<T: Initialize> Clone for Entry<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: Initialize> Entry<T> {
    #[inline(always)]
    pub async fn get(&self) -> Result<TagRef<'_, T>> {
        self.inner.connect().await?;
        let lock = self.inner.lock.lock().await;
        let tag = self.inner.tag();
        Ok(TagRef { tag, lock })
    }
}

#[derive(Debug)]
struct EntryInner<T> {
    state: Arc<EntryState<T>>,
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl<T: Initialize> EntryInner<T> {
    #[inline(always)]
    async fn connect(&self) -> Result<()> {
        let status = *self.state.err_status.wait().await;
        status.into_result()?;
        Ok(())
    }

    #[inline(always)]
    fn id(&self) -> u64 {
        self.state.id
    }

    #[inline(always)]
    fn path(&self) -> &str {
        &self.state.path
    }
    #[inline(always)]
    fn tag(&self) -> &T {
        &self.state.tag
    }
    #[inline(always)]
    fn is_err(&self) -> bool {
        if let Some(status) = self.state.err_status.get() {
            return status.is_err();
        }
        false
    }
}

impl<T> Clone for EntryInner<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            lock: Arc::clone(&self.lock),
        }
    }
}

#[derive(Debug)]
struct EntryHolder<T> {
    inner: EntryInner<T>,
    will_expire: Option<Instant>,
}

impl<T: Initialize> EntryHolder<T> {
    #[inline]
    fn new(id: u64, path: String, tag: T) -> Self {
        Self {
            inner: EntryInner {
                state: Arc::new(EntryState {
                    id,
                    path,
                    tag,
                    err_status: OnceCell::new_notify_all(),
                }),
                lock: Arc::new(tokio::sync::Mutex::new(())),
            },
            will_expire: None,
        }
    }
}

impl<T> Deref for EntryHolder<T> {
    type Target = EntryInner<T>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
struct PoolState<T> {
    ///all tags
    entries: HashMap<u64, EntryHolder<T>>,
    /// ref tags by tag path
    entry_keys: HashMap<String, u64>,
    expirations: BTreeSet<(Instant, u64)>,
    /// for generating next entry id
    next_id: u64,
    shutdown: bool,
}

impl<T: Initialize> PoolState<T> {
    #[inline(always)]
    fn tag_by_path(&self, path: &str) -> Option<&EntryHolder<T>> {
        if let Some(id) = self.entry_keys.get(path) {
            self.entries.get(id)
        } else {
            None
        }
    }

    #[inline]
    fn add_entry(&mut self, path: String) -> plctag::Result<EntryInner<T>> {
        let tag = T::create(path.clone())?;

        let id = self.next_id;
        self.next_id += 1;

        let entry = EntryHolder::new(id, path.clone(), tag);
        let inner = entry.inner.clone();
        self.entry_keys.insert(path, id);
        self.entries.insert(id, entry);
        debug!("PoolState: entry[{}] added", id);
        Ok(inner)
    }

    #[inline]
    fn remove_entry(&mut self, tag_path: &str) -> Option<EntryHolder<T>> {
        if let Some(&id) = self.entry_keys.get(tag_path) {
            if let Some(item) = self.entries.get_mut(&id) {
                let id = item.id();
                self.entry_keys.remove(item.inner.path());
                if let Some(when) = item.will_expire {
                    self.expirations.remove(&(when, id));
                }
                let res = self.entries.remove(&id);
                debug!("PoolState: entry[{}] removed", id);
                return res;
            }
        }
        None
    }

    /// try to make entry alive if exists
    #[inline]
    fn will_alive(&mut self, tag_path: &str) -> Option<&EntryInner<T>> {
        if let Some(&id) = self.entry_keys.get(tag_path) {
            if let Some(item) = self.entries.get_mut(&id) {
                let id = item.id();
                let mut make_alive = true;
                if let Some(status) = item.inner.state.err_status.get() {
                    if status.is_err() {
                        make_alive = false;
                    }
                }

                if make_alive {
                    if let Some(when) = item.will_expire {
                        self.expirations.remove(&(when, id));
                    }
                    item.will_expire = None;
                    debug!("PoolState: entry[{}] will alive", item.id());
                }

                return Some(&item.inner);
            }
        }
        None
    }
}

#[derive(Debug)]
struct Shared<T> {
    state: Mutex<PoolState<T>>,
    options: PoolOptions,
    purge_task: Notify,
    scan_task: ScanTask<T>,
}

impl<T: Initialize> Shared<T> {
    fn new(options: PoolOptions, scan_task: ScanTask<T>) -> Self {
        Self {
            state: Mutex::new(PoolState {
                entries: HashMap::new(),
                entry_keys: HashMap::new(),
                expirations: BTreeSet::new(),
                next_id: 0,
                shutdown: false,
            }),
            options,
            purge_task: Notify::new(),
            scan_task,
        }
    }

    #[inline(always)]
    fn is_shutdown(&self) -> bool {
        let state = self.state.lock();
        state.shutdown
    }

    fn purge_expirations(&self) -> Option<Instant> {
        let state = &mut *self.state.lock();
        if state.shutdown {
            return None;
        }
        let now = Instant::now();
        while let Some(&(when, id)) = state.expirations.iter().next() {
            if when > now {
                return Some(when);
            }
            //expired now
            if let Some(item) = state.entries.remove(&id) {
                state.entry_keys.remove(item.path());
            }
            state.expirations.remove(&(when, id));
        }
        None
    }

    /// register tag entry for expiration
    /// - is_err: true -> set expiration if in error state
    /// - is_err: false -> set expiration if not in error state
    fn will_expire(&self, id: u64, is_err: bool, given_expire: Option<Instant>) {
        let state = &mut *self.state.lock();
        if state.shutdown {
            return;
        }

        let mut will_notify = false;
        if let Some(item) = state.entries.get_mut(&id) {
            //ensure state
            if is_err != item.is_err() {
                return;
            }

            let prev = item.will_expire.map(|when| (when, item.state.id));
            let expire_at = if let Some(v) = given_expire {
                v
            } else {
                Instant::now() + self.options.expire_after
            };
            // insert new
            item.will_expire = Some(expire_at);
            state.expirations.insert((expire_at, id));

            //remove old
            if let Some(key) = prev {
                state.expirations.remove(&key);
            }

            will_notify = true;
        }
        drop(state);

        //send signal
        if will_notify {
            self.purge_task.notify_one();
        }
    }
}

async fn purge_expired_tags_task<T: Initialize>(shared: Arc<Wrapper<T>>) {
    while !shared.is_shutdown() {
        if let Some(when) = shared.purge_expirations() {
            tokio::select! {
                _ = time::sleep_until(when) => {},
                _ = shared.purge_task.notified() => {}
            }
        } else {
            debug!("purge_expired_tags_task: wait for signal");
            shared.purge_task.notified().await;
        }
    }
}

enum CreationMessage<T> {
    Add(Arc<EntryState<T>>),
    Remove(u64),
}

/// with channels, no wait for mutex lock
#[derive(Debug)]
struct ScanTask<T> {
    tx: mpsc::UnboundedSender<CreationMessage<T>>,
    flag: Arc<Notify>,
}

impl<T: Initialize + 'static> ScanTask<T> {
    pub fn new(tx: mpsc::UnboundedSender<CreationMessage<T>>) -> Self {
        Self {
            tx,
            flag: Arc::new(Notify::new()),
        }
    }

    #[inline(always)]
    pub fn add(&self, item: Arc<EntryState<T>>) {
        self.tx.send(CreationMessage::Add(item)).ok();
    }

    #[inline(always)]
    pub fn remove(&self, id: u64) {
        self.tx.send(CreationMessage::Remove(id)).ok();
    }
}

/// message loop task will exit if channel closed;
/// => [`ScanTask`] drops;
/// => [`Shared`] drops;
async fn creation_message_loop<T: Initialize + 'static>(
    mut rx: mpsc::UnboundedReceiver<CreationMessage<T>>,
    flag: Arc<Notify>,
    state: Arc<tokio::sync::Mutex<HashMap<u64, Arc<EntryState<T>>>>>,
) {
    loop {
        if let Some(m) = rx.recv().await {
            match m {
                CreationMessage::Add(item) => {
                    debug!("scan_tags_task: entry[{}] add", item.id);
                    let mut state = state.lock().await;
                    state.insert(item.id, item);
                    flag.notify_one();
                }
                CreationMessage::Remove(id) => {
                    debug!("scan_tags_task: entry[{}] remove", id);
                    let mut state = state.lock().await;
                    state.remove(&id);
                }
            }
        } else {
            //channel closed
            break;
        }
    }
}

/// scan tags creation status;
// task ends when shutdown signal received and no more items to process
async fn scan_tags_task<T: Initialize + 'static>(
    rx: mpsc::UnboundedReceiver<CreationMessage<T>>,
    shared: Arc<Wrapper<T>>,
) {
    let flag = &shared.scan_task.flag;
    let state = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    //recv messages
    task::spawn(creation_message_loop(
        rx,
        Arc::clone(flag),
        Arc::clone(&state),
    ));

    loop {
        debug!("scan_tags_task: scan once");
        let count = {
            let state = &mut *state.lock().await;
            let mut count = state.len();
            if count > 0 {
                let keys: Vec<_> = state.keys().map(|v| *v).collect();
                for id in keys {
                    if let Some(entry) = state.get_mut(&id) {
                        let status = entry.tag.status();
                        if !status.is_pending() {
                            debug!("scan_tags_task: entry[{}] initialized", id);
                            let _ = entry.err_status.set(status);
                            state.remove(&id);

                            // in error state, set expired so it will be removed later
                            if status.is_err() {
                                let when = Instant::now() + shared.options.fault_last;
                                debug!("scan_tags_task: entry[{}] will expire", id);
                                shared.will_expire(id, true, Some(when));
                            }
                        }
                    }
                }
                count = state.len();
            }
            drop(state);
            count
        };
        // no more items, wait for signal
        if count == 0 {
            if shared.is_shutdown() {
                break;
            }
            debug!("scan_tags_task: wait for signal");
            flag.notified().await;
        } else {
            time::sleep(Duration::from_millis(1)).await;
        }
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicU8, Ordering};

    use super::*;
    use plctag::ffi;

    struct FakeTag {
        count: AtomicU8,
        path: String,
    }

    impl FakeTag {
        fn count(&self) -> u8 {
            self.count.load(Ordering::Acquire)
        }
    }

    impl Initialize for FakeTag {
        fn create(path: String) -> plctag::Result<Self>
        where
            Self: Sized,
        {
            if path.starts_with("err") {
                Err(Status::Err(ffi::PLCTAG_ERR_TIMEOUT).into())
            } else {
                Ok(FakeTag {
                    count: AtomicU8::new(0),
                    path,
                })
            }
        }

        fn status(&self) -> Status {
            let count = self.count.load(Ordering::Acquire);
            if count < 10 {
                self.count.store(count + 1, Ordering::Release);
                Status::Pending
            } else if self.path.contains("timeout") {
                Status::Err(ffi::PLCTAG_ERR_TIMEOUT).into()
            } else {
                Status::Ok
            }
        }
    }

    #[test]
    fn error_at_first() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let res = pool.entry("err").await;
            assert!(res.is_err());

            assert_eq!(pool.len(), 0);
        });
        Ok(())
    }

    #[test]
    fn error_at_last() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let res = pool.entry("timeout").await;
            let entry = res.unwrap();
            assert_eq!(pool.len(), 1);
            let res = entry.get().await;
            assert!(res.is_err());
            drop(res);
            drop(entry);
            assert_eq!(pool.len(), 1);

            drop(pool);
        });
        Ok(())
    }

    #[test]
    fn only_one_instance() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let task1 = {
                let pool = pool.clone();
                task::spawn(async move {
                    let res = pool.entry("one_tag").await;
                    assert!(res.is_ok());
                })
            };

            let task2 = {
                let pool = pool.clone();
                task::spawn(async move {
                    let res = pool.entry("one_tag").await;
                    assert!(res.is_ok());
                })
            };

            let _ = tokio::join!(task1, task2);

            assert_eq!(pool.len(), 1);
        });
        Ok(())
    }

    #[test]
    fn more_instances() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let task1 = {
                let pool = pool.clone();
                task::spawn(async move {
                    let res = pool.entry("one_tag").await;
                    assert!(res.is_ok());
                })
            };

            let task2 = {
                let pool = pool.clone();
                task::spawn(async move {
                    let res = pool.entry("one_tag").await;
                    assert!(res.is_ok());
                })
            };

            let task3 = {
                let pool = pool.clone();
                task::spawn(async move {
                    let res = pool.entry("another").await;
                    assert!(res.is_ok());
                })
            };

            let _ = tokio::join!(task1, task2, task3);
            assert_eq!(pool.len(), 2);
        });
        Ok(())
    }

    #[test]
    fn test_remove() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let res = pool.entry("one_tag").await;
            assert!(res.is_ok());
            assert_eq!(pool.len(), 1);
            pool.remove("one_tag");
            assert_eq!(pool.len(), 0);
        });
        Ok(())
    }

    #[test]
    fn test_expire_for_normal() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = PoolOptions::default()
                .expire_after(Duration::from_millis(100))
                .create();
            let res = pool.entry("one_tag").await;
            assert!(res.is_ok());
            assert_eq!(pool.len(), 1);
            drop(res);
            time::sleep(Duration::from_millis(150)).await;
            assert_eq!(pool.len(), 0);
        });
        Ok(())
    }

    #[test]
    fn test_expire_for_error() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = PoolOptions::default()
                .fault_last(Duration::from_millis(100))
                .create();
            let res = pool.entry("timeout").await;
            let entry = res.unwrap();
            assert_eq!(pool.len(), 1);
            let res = entry.get().await;
            assert!(res.is_err());
            drop(res);
            drop(entry);
            time::sleep(Duration::from_millis(150)).await;
            assert_eq!(pool.len(), 0);
        });
        Ok(())
    }
}

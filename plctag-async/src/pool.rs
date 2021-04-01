use crate::{Error, Result, TagRef};

use plctag::{RawTag, Status};

use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::{
    cell::UnsafeCell,
    collections::{BTreeSet, HashMap},
    future::Future,
    ops::Deref,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

use tokio::{
    sync::Notify,
    task,
    time::{self, Duration, Instant},
};

#[doc(hidden)]
pub trait Initialize: Send + Sync {
    fn create(path: String) -> Result<Self>
    where
        Self: Sized;
    fn status(&self) -> Status;
}

impl Initialize for RawTag {
    fn create(path: String) -> Result<Self> {
        let tag = RawTag::new(path, 0)?;
        Ok(tag)
    }

    fn status(&self) -> Status {
        RawTag::status(self)
    }
}

pub struct Pool<T: Initialize> {
    shared: Arc<Shared<T>>,
}

impl<T: Initialize> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: Initialize + 'static> Pool<T> {
    pub fn new() -> Self {
        let shared = Arc::new(Shared::new());
        {
            task::spawn(scan_tags_task(shared.clone()));
        }
        Self { shared }
    }

    /// get or create tag, returns after created
    pub async fn entry(&self, options: impl AsRef<str>) -> Result<Entry<T>> {
        let mut pending = false;
        let mut state = {
            let reader = self.shared.state.upgradable_read();
            if let Some(tag) = reader.get_tag_by_path(options.as_ref()) {
                tag.clone()
            } else {
                // create if not exist
                let state = {
                    let mut writer = RwLockUpgradableReadGuard::upgrade(reader);
                    writer.add_tag(options.as_ref().to_owned())
                };
                pending = true;
                state
            }
        };

        if pending {
            create_tag_task(self.shared.clone(), &mut state).await;
        }
        state.connect().await?;
        Ok(state)
    }

    /// remove tag from pool
    pub fn remove(&self, options: &str) -> Option<Entry<T>> {
        let reader = self.shared.state.upgradable_read();
        if let Some(tag) = reader.get_tag_by_path(options) {
            let id = tag.id();
            let mut writer = RwLockUpgradableReadGuard::upgrade(reader);
            writer.tag_keys.remove(options);
            writer.tags.remove(&id)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        let reader = self.shared.state.read();
        reader.tags.len()
    }

    pub async fn for_each<F, Fut>(&self, f: F) -> Result<()>
    where
        F: Fn(Entry<T>) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let reader = self.shared.state.read();
        let tags = reader.tags.clone();
        drop(reader);
        for (_, entry) in tags {
            f(entry).await?;
        }
        Ok(())
    }
}

impl<T: Initialize> Drop for Pool<T> {
    fn drop(&mut self) {
        // - this ref
        // - scan task
        if Arc::strong_count(&self.shared) == 2 {
            {
                let mut writer = self.shared.state.write();
                writer.shutdown = true;
            }
            self.shared.scan_task.notify_one();
        }
    }
}

impl<T: Initialize + 'static> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

const CREATION_EMPTY: u8 = 0;
const CREATION_HAS_INSTANCE: u8 = 1;
const CREATION_DONE: u8 = 2;

struct CreationStatus(u8);
impl CreationStatus {
    #[inline(always)]
    fn load(flag: &AtomicU8, order: Ordering) -> Self {
        Self(flag.load(order))
    }
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0 == CREATION_EMPTY
    }
    #[inline(always)]
    fn has_instance(&self) -> bool {
        self.0 & CREATION_HAS_INSTANCE == CREATION_HAS_INSTANCE
    }
    #[inline(always)]
    fn is_done(&self) -> bool {
        self.0 & CREATION_DONE == CREATION_DONE
    }
}

struct EntryInner<T> {
    /// unique id for this instance
    id: u64,
    /// tag options
    path: String,
    tag: UnsafeCell<Option<T>>,
    err: UnsafeCell<Result<()>>,
    status: AtomicU8,
    create_task: Notify,
}

unsafe impl<T> Send for EntryInner<T> {}
unsafe impl<T> Sync for EntryInner<T> {}

pub struct Entry<T> {
    inner: Arc<EntryInner<T>>,
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl<T: Initialize> Clone for Entry<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            lock: Arc::clone(&self.lock),
        }
    }
}

impl<T: Initialize> Entry<T> {
    #[inline]
    fn new(id: u64, path: String) -> Self {
        Self {
            inner: Arc::new(EntryInner {
                id,
                path,
                tag: UnsafeCell::new(None),
                err: UnsafeCell::new(Ok(())),
                status: AtomicU8::new(CREATION_EMPTY),
                create_task: Notify::new(),
            }),
            lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    #[inline(always)]
    fn id(&self) -> u64 {
        self.inner.id
    }

    #[inline(always)]
    fn path(&self) -> &str {
        &self.inner.path
    }
    #[inline]
    fn get_tag(&self) -> &T {
        if self.has_instance() {
            self.get_tag_unchecked()
        } else {
            panic!("bad usage, tag not ready yet")
        }
    }
    #[inline]
    fn get_tag_unchecked(&self) -> &T {
        if let Some(res) = unsafe { &*self.inner.tag.get() } {
            res
        } else {
            panic!("bad usage, tag not ready yet")
        }
    }
    #[inline(always)]
    fn has_instance(&self) -> bool {
        //relax
        let status = CreationStatus::load(&self.inner.status, Ordering::Relaxed);
        if status.has_instance() {
            return true;
        }
        let status = CreationStatus::load(&self.inner.status, Ordering::Acquire);
        status.has_instance()
    }
    #[inline(always)]
    fn is_done(&self) -> bool {
        //relax
        let status = CreationStatus::load(&self.inner.status, Ordering::Relaxed);
        if status.is_done() {
            return true;
        }
        let status = CreationStatus::load(&self.inner.status, Ordering::Acquire);
        status.is_done()
    }
    #[inline(always)]
    fn check_status(&self) -> Option<Status> {
        if !self.has_instance() {
            return None;
        }
        Some(self.get_tag_unchecked().status())
    }

    fn set_tag(&mut self, res: Result<T>) {
        match res {
            Ok(tag) => {
                let status = CreationStatus::load(&self.inner.status, Ordering::Acquire);
                if !status.is_empty() {
                    return;
                }
                let holder = unsafe { &mut *self.inner.tag.get() };
                if holder.is_none() {
                    *holder = Some(tag);
                    self.inner
                        .status
                        .store(CREATION_HAS_INSTANCE, Ordering::Release);
                }
            }
            Err(e) => {
                self.set_err(Err(e));
            }
        };
    }

    fn set_err(&mut self, res: Result<()>) {
        let status = CreationStatus::load(&self.inner.status, Ordering::Acquire);
        if status.is_done() {
            return;
        }

        if let Err(Error::TagError(ref e)) = res {
            if e.is_pending() {
                panic!("should not be pending status here");
            }
        }

        // set err
        let holder = unsafe { &mut *self.inner.err.get() };
        *holder = res;

        //set status
        self.inner
            .status
            .store(status.0 | CREATION_DONE, Ordering::Release);

        //notify awaiters
        self.inner.create_task.notify_waiters();
    }

    #[inline(always)]
    async fn connect(&self) -> Result<()> {
        if !self.is_done() {
            self.inner.create_task.notified().await;
        }
        let res = unsafe { &*self.inner.err.get() };
        res.clone()
    }

    pub async fn get(&self) -> Result<TagRef<'_, T>> {
        self.connect().await?;
        let lock = self.lock.lock().await;
        let raw = self.get_tag_unchecked();
        Ok(TagRef { tag: raw, lock })
    }
}

struct State<T: Initialize> {
    tags: HashMap<u64, Entry<T>>,
    /// ref tags by tag path
    tag_keys: HashMap<String, u64>,
    /// ref tags when scanning tag status
    creation: BTreeSet<(Instant, u64)>,
    next_id: u64,
    shutdown: bool,
}

impl<T: Initialize> State<T> {
    #[inline(always)]
    fn get_tag(&self, id: u64) -> Option<&Entry<T>> {
        self.tags.get(&id)
    }

    #[inline(always)]
    fn get_tag_by_path(&self, path: &str) -> Option<&Entry<T>> {
        if let Some(id) = self.tag_keys.get(path) {
            self.tags.get(id)
        } else {
            None
        }
    }
    #[inline(always)]
    fn add_tag(&mut self, path: String) -> Entry<T> {
        let id = self.next_id;
        self.next_id += 1;

        let state = Entry::new(id, path.clone());
        {
            let state = state.clone();
            self.tag_keys.insert(path, id);
            self.tags.insert(id, state);
        }
        state
    }
}

struct Shared<T: Initialize> {
    state: RwLock<State<T>>,
    scan_task: Notify,
}

impl<T: Initialize> Shared<T> {
    fn new() -> Self {
        Self {
            state: RwLock::new(State {
                tags: HashMap::new(),
                tag_keys: HashMap::new(),
                creation: BTreeSet::new(),
                next_id: 0,
                shutdown: false,
            }),
            scan_task: Notify::new(),
        }
    }

    fn is_shutdown(&self) -> bool {
        let state = self.state.read();
        state.shutdown
    }

    /// returns remaining tags count
    fn scan_once(&self) -> Option<Instant> {
        let reader = self.state.upgradable_read();
        if reader.shutdown {
            return None;
        }

        let mut writer = RwLockUpgradableReadGuard::upgrade(reader);
        let now = Instant::now();
        while let Some(&(when, id)) = writer.creation.iter().next() {
            if when > now {
                return Some(when);
            }
            //tag still exists?
            if let Some(tag) = writer.tags.get_mut(&id) {
                //check tag status()
                if let Some(status) = tag.check_status() {
                    //tag does exists and has status
                    if status.is_pending() {
                        //for further checking
                        let next_time = now + Duration::from_millis(5);
                        writer.creation.insert((next_time, id));
                    } else {
                        //into final state if OK or Error
                        if status.is_ok() {
                            tag.set_err(Ok(()));
                        } else {
                            tag.set_err(Err(status.into()));
                        }
                    }
                }
            };
            writer.creation.remove(&(when, id));
        }
        None
    }
}

async fn scan_tags_task<T: Initialize>(shared: Arc<Shared<T>>) {
    while !shared.is_shutdown() {
        if let Some(when) = shared.scan_once() {
            tokio::select! {
                _ = time::sleep_until(when) => {}
                _ = shared.scan_task.notified() => {}
            }
        } else {
            shared.scan_task.notified().await;
        }
    }
}

async fn create_tag_task<T: Initialize + 'static>(shared: Arc<Shared<T>>, state: &mut Entry<T>) {
    let res = {
        let path = state.path().to_owned();
        task::spawn_blocking(move || T::create(path))
            .await
            .map_err(|e| e.into())
            .and_then(|res| res)
    };

    let has_err = res.is_err();
    state.set_tag(res);

    {
        let mut writer = shared.state.write();
        if has_err {
            writer.tag_keys.remove(state.path());
            writer.tags.remove(&state.id());
        } else {
            //for further checking
            let next_time = Instant::now() + Duration::from_millis(5);
            writer.creation.insert((next_time, state.id()));
        }
    }

    if !has_err {
        shared.scan_task.notify_one();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use plctag_sys as ffi;

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
        fn create(path: String) -> Result<Self>
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

            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
                assert!(reader.tags.is_empty());
                assert!(reader.tag_keys.is_empty());
            }

            drop(pool);
        });
        Ok(())
    }

    #[test]
    fn error_at_last() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool: Pool<FakeTag> = Pool::new();
            let res = pool.entry("timeout").await;
            assert!(res.is_err());

            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
            }

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

            tokio::join!(task1, task2);
            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
                assert!(reader.tags.len() == 1);
                assert!(reader.tag_keys.len() == 1);
            }
            drop(pool);
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

            tokio::join!(task1, task2, task3);
            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
                assert!(reader.tags.len() == 2);
                assert!(reader.tag_keys.len() == 2);
            }
            drop(pool);
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
            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
                assert!(reader.tags.len() == 1);
                assert!(reader.tag_keys.len() == 1);
            }
            pool.remove("one_tag");
            {
                let reader = pool.shared.state.read();
                assert!(reader.creation.is_empty());
                assert!(reader.tags.len() == 0);
                assert!(reader.tag_keys.len() == 0);
            }
            drop(pool);
        });
        Ok(())
    }
}

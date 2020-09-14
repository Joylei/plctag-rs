//! use plctag::future::prelude::*;
//! use std::sync::Arc;
//! use tokio::task;
//! use tokio::{self, runtime::Runtime};
//!
//! static PLC_HOST: &str = "192.168.1.120";
//!
//! #[derive(Clone)]
//! struct MyTagBuilder {
//!     name: String,
//!     path: String,
//! }
//!
//! impl MyTagBuilder {
//!     pub fn new(name: &str, path: &str) -> Self {
//!         Self {
//!             name: name.to_owned(),
//!             path: path.to_owned(),
//!         }
//!     }
//! }
//!
//! impl TagOptions for MyTagBuilder {
//!     fn host(&self) -> String {
//!         PLC_HOST.to_owned()
//!     }
//!
//!     fn name(&self) -> String {
//!         self.name.clone()
//!     }
//!
//!     fn path(&self) -> String {
//!         self.path.clone()
//!     }
//! }
//!
//! struct PingPong {}
//!
//! impl Operation for PingPong {
//!     fn id(&self) -> usize {
//!         1
//!     }
//!
//!     /// get & set value here
//!     fn run(&self, ctx: Processor) -> tokio::task::JoinHandle<Result<()>> {
//!         task::spawn(async move {
//!             let res = ctx.find_tag("MyTag1");
//!             let tag1 = match res {
//!                 Some(tag) => tag,
//!                 _ => return Ok(()),
//!             };
//!             let res = ctx.find_tag("MyTag2");
//!             let tag2 = match res {
//!                 Some(tag) => tag,
//!                 _ => return Ok(()),
//!             };
//!
//!             //read write whatever type
//!             let size = tag1.size().await?;
//!             let buf = Buf::new(size as usize);
//!             //let buf = unsafe { Pin::new_unchecked(&mut buf) };
//!             let (buf, _) = tag1.get_bytes(buf).await?;
//!             tag2.set_bytes(buf).await?;
//!             Ok(())
//!         })
//!     }
//!
//!     fn expired(&self) -> bool {
//!         false
//!     }
//! }
//!
//! fn main() {
//!     let config = ControllerOptions::new(PLC_HOST);
//!     let controller = Arc::new(Controller::from(config));
//!     let controller1 = Arc::clone(&controller);
//!
//!     let mut rt = Runtime::new().unwrap();
//!     rt.block_on(async move {
//!         let _task = tokio::spawn(async move {
//!             controller.scan().await
//!         });
//!
//!         //add tags
//!         let tag1 = MyTagBuilder::new("MyTag1", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16");
//!         let tag2 = MyTagBuilder::new("MyTag2", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=1&elem_size=16");
//!
//!         let res1 = controller1.ensure_tag(tag1.clone()).await;
//!         assert!(res1.is_some());
//!         let res2 = controller1.ensure_tag(tag2.clone()).await;
//!         assert!(res2.is_some());
//!
//!         //post operations to controller
//!         for _ in 0..1000 {
//!             controller1.post(PingPong {}).await;
//!         }
//!
//!         drop(_task);
//!     });
//! }

#![cfg(feature = "controller")]
use super::tag::AsyncTag;
pub use crate::controller::{ControllerOptions, TagOptions};
use crate::{error::Error, Result, Status};
use futures::future::join_all;
use std::{
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time};

pub trait Operation {
    /// unique id of the operation
    /// the `Controller` will drop operations if their ids are equal, and only keep latest one
    fn id(&self) -> usize;
    /// only  get/set value in the operation
    fn run(&self, ctx: Processor) -> tokio::task::JoinHandle<Result<()>>;

    /// check expiration, the `Controller` skip processing the operation if expired
    fn expired(&self) -> bool;
}

pub trait Context {
    fn find_tag(&self, name: &str) -> Option<Arc<TagEntry>>;
}
/// operation context
pub struct Processor {
    tags: Arc<HashMap<String, Arc<TagEntry>>>,
    host: String,
}

impl Processor {
    #[inline]
    pub(crate) fn new(host: impl AsRef<str>, tags: Arc<HashMap<String, Arc<TagEntry>>>) -> Self {
        Self {
            tags,
            host: host.as_ref().to_owned(),
        }
    }

    #[inline]
    pub(crate) async fn process(&self, messages: Vec<Box<dyn Operation + Send + Sync>>) {
        for op in &messages {
            let res = op.run(self.clone()).await;
            if let Err(e) = res {
                error!(
                    "controller {}: failed to run operation {}: {}",
                    self.host,
                    op.id(),
                    e
                );
            }
        }
    }
}

impl Clone for Processor {
    fn clone(&self) -> Self {
        Self {
            tags: Arc::clone(&self.tags),
            host: self.host.to_owned(),
        }
    }
}

impl Context for Processor {
    #[inline]
    fn find_tag(&self, name: &str) -> Option<Arc<TagEntry>> {
        let key = generate_key(&self.host, name);
        self.tags.get(&key).map(|tag| Arc::clone(tag))
    }
}

unsafe impl Send for Processor {}
unsafe impl Sync for Processor {}

/// `Controller` will take care tags's read and write.
/// You only need to post `Operation`s to `Controller`.
pub struct Controller {
    tags: Arc<Mutex<HashMap<String, Arc<TagEntry>>>>,
    messages: Arc<Mutex<HashMap<usize, Box<dyn Operation + Send + Sync>>>>,
    opts: ControllerOptions,
}

impl Clone for Controller {
    fn clone(&self) -> Self {
        Self {
            tags: Arc::clone(&self.tags),
            messages: Arc::clone(&self.messages),
            opts: ControllerOptions {
                host: self.opts.host.clone(),
                ..self.opts
            },
        }
    }
}

impl From<ControllerOptions> for Controller {
    #[inline]
    fn from(opts: ControllerOptions) -> Self {
        Self {
            tags: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
            opts,
        }
    }
}

impl Controller {
    #[inline]
    pub fn host(&self) -> &str {
        &self.opts.host
    }

    #[inline]
    pub fn new(host: impl AsRef<str>) -> Self {
        let opts = ControllerOptions::new(host);
        Self::from(opts)
    }

    pub async fn ensure_tag(&self, opts: impl TagOptions) -> Option<Arc<TagEntry>> {
        let key = self.generate_key(&opts.name());
        let key1 = key.clone();

        let mut map = self.tags.lock().await;
        if !map.contains_key(&key) {
            let res = TagEntry::new(opts).await;
            match res {
                Ok(tag) => {
                    let tag = Arc::new(tag);
                    let tag1 = Arc::clone(&tag);
                    map.insert(key, tag);
                    return Some(tag1);
                }
                Err(e) => {
                    error!(
                        "controller {} - failed to create tag {}: {}",
                        self.opts.host, &key1, e
                    );
                    return None;
                }
            }
        }
        let tag = map.get(&key1).unwrap();
        Some(Arc::clone(tag))
    }

    #[inline]
    pub async fn scan(&self) {
        loop {
            self.scan_once().await;
            time::delay_for(self.opts.scan_interval).await;
        }
    }

    async fn scan_once(&self) {
        //check tag ready
        let all_tags = self.check_ready_tags().await;
        if all_tags.len() == 0 {
            trace!("controller {} - no ready tags", &self.opts.host);
            return;
        }
        let ready_tags = self.read_all(all_tags).await;
        if ready_tags.len() == 0 {
            return;
        }
        let ready_tags = Arc::new(ready_tags);
        self.process_messages(Arc::clone(&ready_tags)).await;

        self.write_all(Arc::clone(&ready_tags)).await;
    }

    #[inline]
    async fn take_messages(&self) -> Vec<Box<dyn Operation + Send + Sync>> {
        let map = &mut *self.messages.lock().await;
        let keys: Vec<usize> = map.keys().map(|x| x.clone()).collect();
        let mut res = vec![];
        for key in keys.iter() {
            if let Some(v) = map.remove(key) {
                if v.expired() {
                    continue;
                }
                res.push(v);
            }
        }
        res
    }

    #[inline]
    async fn process_messages(&self, ready_tags: Arc<HashMap<String, Arc<TagEntry>>>) {
        let messages = self.take_messages().await;
        let processor = Processor::new(&self.opts.host, ready_tags);
        processor.process(messages).await;
    }

    async fn check_ready_tags(&self) -> HashMap<String, Arc<TagEntry>> {
        let tags: HashMap<String, Arc<TagEntry>> = {
            let map = self.tags.lock().await;
            map.clone()
        };

        //check tag ready
        let mut to_remove = vec![];
        let mut ready_tags: HashMap<String, Arc<TagEntry>> = HashMap::new();
        for (key, tag) in tags {
            match tag.check_ready(self.opts.create_timeout).await {
                Ok(ready) => {
                    if ready {
                        ready_tags.insert(key, tag);
                    }
                }
                Err(e) => {
                    error!(
                        "controller {} - failed to create tag {}: {}",
                        self.opts.host,
                        tag.name(),
                        e
                    );
                    to_remove.push(key);
                }
            }
        }

        if to_remove.len() > 0 {
            let tags = &mut *self.tags.lock().await;
            for key in to_remove {
                tags.remove(&key);
            }
        }

        ready_tags
    }

    async fn write_all(&self, source: Arc<HashMap<String, Arc<TagEntry>>>) {
        let mut futures = vec![];
        for tag in source.values() {
            let tag1 = Arc::clone(tag);
            let tag2 = Arc::clone(tag);
            let res = async move {
                let res = time::timeout(self.opts.write_timeout, tag1.write()).await;
                (tag2, res)
            };
            futures.push(res);
        }

        let results = join_all(futures).await;
        for (tag, res) in results {
            match res {
                Ok(status) => {
                    if status.is_err() {
                        error!(
                            "controller {} - failed to write tag {}: {}",
                            self.opts.host,
                            tag.name(),
                            status.decode()
                        );
                    }
                }
                Err(_) => {
                    error!(
                        "controller {} - timeout to write tag {}",
                        self.opts.host,
                        tag.name()
                    );
                }
            }
        }
    }

    pub async fn read_all(
        &self,
        ready_tags: HashMap<String, Arc<TagEntry>>,
    ) -> HashMap<String, Arc<TagEntry>> {
        let mut futures = vec![];
        for tag in ready_tags.values() {
            let tag1 = Arc::clone(tag);
            let tag2 = Arc::clone(tag);
            let res = async move {
                let res = time::timeout(self.opts.read_timeout, tag1.read()).await;
                (tag2, res)
            };
            futures.push(res);
        }
        let results = join_all(futures).await;
        results
            .iter()
            .filter(|(tag, res)| match res {
                Ok(status) => {
                    if status.is_err() {
                        error!(
                            "controller {} - failed to read tag {}: {}",
                            self.opts.host,
                            tag.name(),
                            status.decode()
                        );
                        return false;
                    }
                    true
                }
                Err(_) => {
                    error!(
                        "controller {} - timeout to write tag {}",
                        self.opts.host,
                        tag.name()
                    );
                    false
                }
            })
            .map(|(tag, _)| (self.generate_key(tag.name()), Arc::clone(tag)))
            .collect()
    }

    /// you need to call `ensure_tag` to create tag if not exist
    pub async fn post(&self, op: impl Operation + Send + Sync + 'static) {
        let messages = &mut *self.messages.lock().await;
        let key = op.id();
        //TODO: use policy
        messages.entry(key).or_insert(Box::new(op));
    }

    #[inline]
    pub(crate) fn generate_key(&self, tag_name: &str) -> String {
        generate_key(&self.opts.host, tag_name)
    }
}

#[inline]
fn generate_key(host: &str, tag_name: &str) -> String {
    format!("{},{}", host, tag_name)
}

impl Drop for Controller {
    fn drop(&mut self) {
        let mut tags = futures::executor::block_on(self.tags.lock());
        tags.clear();
    }
}

#[derive(Debug)]
pub struct TagEntry {
    host: String,
    name: String,
    raw: AsyncTag,
    /// use atomicbool, unsafecell to remove mut self constrain
    ready: AtomicBool,
    created: Instant,
}

impl Deref for TagEntry {
    type Target = AsyncTag;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl DerefMut for TagEntry {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

impl TagEntry {
    #[inline]
    pub async fn new(opts: impl TagOptions) -> Result<Self> {
        Ok(Self {
            host: opts.host(),
            name: opts.name(),
            raw: AsyncTag::new(&opts.path()).await?,
            ready: AtomicBool::new(false),
            created: Instant::now(),
        })
    }

    #[inline]
    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub async fn check_ready(&self, timeout: Duration) -> Result<bool> {
        if self.ready() {
            return Ok(true);
        }
        let status = self.raw.status().await;
        if status.is_err() {
            return Err(Error::from(status));
        }
        if status.is_ok() {
            self.ready.store(true, Ordering::Relaxed);
            info!(
                "controller {} - tag {} gets ready now",
                self.host, self.name
            );
            return Ok(true);
        }
        //pending
        //check timeout
        if self.created.elapsed() > timeout {
            return Err(Error::from(Status::new(-32)));
        }
        Ok(false)
    }

    /// tag name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TagEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Eq for TagEntry {}

impl PartialEq for TagEntry {
    #[inline]
    fn eq(&self, other: &TagEntry) -> bool {
        self.host.eq(&other.host) && self.name.eq(&other.name)
    }
}

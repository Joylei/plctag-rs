//! use controller to scan tags
//!
//! # Examples
//! ```rust,ignore
//! use plctag::{controller::*, Result};
//! use std::sync::Arc;
//! use std::thread;
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
//!     fn run<'a>(&self, ctx: &'a dyn Context) -> Result<()> {
//!         let res = ctx.find_tag("MyTag1");
//!         let tag1 = match res {
//!             Some(tag) => tag,
//!             _ => return Ok(()),
//!         };
//!         let res = ctx.find_tag("MyTag2");
//!         let tag2 = match res {
//!             Some(tag) => tag,
//!             _ => return Ok(()),
//!         };
//!
//!         //read write whatever type
//!         let size = tag1.size()?;
//!         let mut buf: Vec<u8> = vec![0; size as usize];
//!         tag1.get_bytes(&mut buf)?;
//!         tag2.set_bytes(&buf)?;
//!         Ok(())
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
//!     //run controller in another thread
//!     let thread1 = thread::spawn(move || controller.scan());
//!
//!     //add tags
//!     let tag1 = MyTagBuilder::new("MyTag1", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16");
//!     let tag2 = MyTagBuilder::new("MyTag2", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=1&elem_size=16");

//!     let res1 = controller1.ensure_tag(tag1.clone());
//!     assert!(res1.is_some());
//!     let res2 = controller1.ensure_tag(tag2.clone());
//!     assert!(res2.is_some());
//!
//!     //post operations to controller
//!     for _ in 0..1000 {
//!         controller1.post(PingPong {});
//!     }
//!
//!     thread1.join().unwrap();
//! }
//! ```

#![cfg(feature = "controller")]

use crate::{error::Error, RawTag, Result, Status};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

/// operation send to controller.
/// normally you should  only  get/set value in the operation
pub trait Operation {
    /// unique id of the operation
    /// the `Controller` will drop operations if their ids are equal, and only keep latest one
    fn id(&self) -> usize;
    /// only  get/set value in the operation
    fn run<'a>(&self, ctx: &'a dyn Context) -> Result<()>;
    /// check expiration, the `Controller` skip processing the operation if expired
    fn expired(&self) -> bool;
}

pub trait Context<'a> {
    fn find_tag(&self, name: &str) -> Option<&'a Arc<TagEntry>>;
}

/// operation context
struct Processor<'a> {
    tags: &'a HashMap<String, Arc<TagEntry>>,
    controller: &'a Controller,
}

impl<'a> Processor<'a> {
    #[inline]
    pub(crate) fn new(
        controller: &'a Controller,
        tags: &'a HashMap<String, Arc<TagEntry>>,
    ) -> Self {
        Self { tags, controller }
    }

    #[inline]
    pub(crate) fn process(&self, messages: Vec<Box<dyn Operation + Send>>) {
        for op in &messages {
            let res = op.run(self);
            if let Err(e) = res {
                error!(
                    "controller {}: failed to run operation {}: {}",
                    self.controller.host(),
                    op.id(),
                    e
                );
            }
        }
    }
}

impl<'a> Context<'a> for Processor<'a> {
    #[inline]
    fn find_tag(&self, name: &str) -> Option<&'a Arc<TagEntry>> {
        let key = self.controller.generate_key(name);
        self.tags.get(&key)
    }
}

/// build tag
pub trait TagOptions {
    /// plc gateway host
    fn host(&self) -> String;
    /// tag name
    fn name(&self) -> String;
    /// full path of  tag, see `libplctag`
    fn path(&self) -> String;
}

/// build controller
pub struct ControllerOptions {
    pub(crate) host: String,
    pub(crate) scan_interval: Duration,
    pub(crate) create_timeout: Duration,
    pub(crate) read_timeout: Duration,
    pub(crate) write_timeout: Duration,
    pub(crate) poll_interval: Duration,
}

impl ControllerOptions {
    #[inline]
    pub fn new(host: impl AsRef<str>) -> Self {
        Self {
            host: host.as_ref().to_owned(),
            scan_interval: Duration::from_millis(20),
            create_timeout: Duration::from_millis(300),
            read_timeout: Duration::from_millis(200),
            write_timeout: Duration::from_millis(200),
            poll_interval: Duration::from_millis(1),
        }
    }

    #[inline]
    pub fn host(mut self, host: impl AsRef<str>) -> Self {
        self.host = host.as_ref().to_owned();
        self
    }
    #[inline]
    pub fn scan_interval(mut self, scan_interval: Duration) -> Self {
        self.scan_interval = scan_interval;
        self
    }
    #[inline]
    pub fn create_timeout(mut self, create_timeout: Duration) -> Self {
        self.create_timeout = create_timeout;
        self
    }
    #[inline]
    pub fn read_timeout(mut self, read_timeout: Duration) -> Self {
        self.read_timeout = read_timeout;
        self
    }
    #[inline]
    pub fn write_timeout(mut self, write_timeout: Duration) -> Self {
        self.write_timeout = write_timeout;
        self
    }

    #[inline]
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }
}

/// `Controller` will take care tags's read and write.
/// You only need to post `Operation`s to `Controller`.
pub struct Controller {
    tags: Arc<Mutex<HashMap<String, Arc<TagEntry>>>>,
    messages: Arc<Mutex<HashMap<usize, Box<dyn Operation + Send>>>>,
    opts: ControllerOptions,
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

    pub fn ensure_tag(&self, opts: impl TagOptions) -> Option<Arc<TagEntry>> {
        let key = self.generate_key(&opts.name());
        let key1 = key.clone();

        let mut map = self.tags.lock();
        if !map.contains_key(&key) {
            let res = TagEntry::new(opts);
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
    pub fn scan(&self) {
        loop {
            self.scan_once();
            thread::sleep(self.opts.scan_interval)
        }
    }

    fn scan_once(&self) {
        //check tag ready
        let all_tags = self.check_ready_tags();
        if all_tags.len() == 0 {
            trace!("controller {} - no ready tags", &self.opts.host);
            return;
        }
        let ready_tags = self.read_all(all_tags);
        if ready_tags.len() == 0 {
            return;
        }
        self.process_messages(&ready_tags);

        self.write_all(ready_tags);
    }

    #[inline]
    fn take_messages(&self) -> Vec<Box<dyn Operation + Send>> {
        let map = &mut *self.messages.lock();
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
    fn process_messages(&self, ready_tags: &HashMap<String, Arc<TagEntry>>) {
        let messages = self.take_messages();
        let processor = Processor::new(self, ready_tags);
        processor.process(messages);
    }

    fn check_ready_tags(&self) -> HashMap<String, Arc<TagEntry>> {
        let tags: HashMap<String, Arc<TagEntry>> = {
            let map = self.tags.lock();
            map.clone()
        };

        //check tag ready
        let mut to_remove = vec![];
        let mut ready_tags: HashMap<String, Arc<TagEntry>> = HashMap::new();
        for (key, tag) in tags {
            match tag.check_ready(self.opts.create_timeout) {
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
            let tags = &mut *self.tags.lock();
            for key in to_remove {
                tags.remove(&key);
            }
        }

        ready_tags
    }

    fn write_all(&self, source: HashMap<String, Arc<TagEntry>>) {
        let mut remaining = source;
        let now = Instant::now();
        let mut first = true;
        loop {
            if now.elapsed() > self.opts.write_timeout {
                break;
            }
            remaining = remaining
                .into_iter()
                .filter(|(_key, tag)| {
                    let status = if first {
                        first = false;
                        tag.write(0)
                    } else {
                        tag.status()
                    };
                    match status {
                        Status::Ok => false,
                        Status::Pending => true,
                        _ => {
                            error!(
                                "controller {} - failed to write tag {}: {}",
                                self.opts.host,
                                tag.name(),
                                status.decode()
                            );
                            false
                        }
                    }
                })
                .collect();

            // stop loop if no pending tags
            if remaining.len() == 0 {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        //abort all remaining tags
        for tag in remaining.values() {
            error!(
                "controller {} - timeout to write tag {}",
                self.opts.host,
                tag.name()
            );
            let _ = tag.abort();
        }
    }

    pub fn read_all(
        &self,
        ready_tags: HashMap<String, Arc<TagEntry>>,
    ) -> HashMap<String, Arc<TagEntry>> {
        let mut remaining = ready_tags;
        let now = Instant::now();
        let mut first = true;
        let mut res: Vec<Arc<TagEntry>> = vec![];
        loop {
            if now.elapsed() > self.opts.read_timeout {
                break;
            }
            remaining = remaining
                .into_iter()
                .filter(|(_key, tag)| {
                    let status = if first {
                        first = false;
                        tag.read(0)
                    } else {
                        tag.status()
                    };
                    match status {
                        Status::Ok => {
                            res.push(Arc::clone(tag));
                            false
                        }
                        Status::Pending => true,
                        _ => {
                            error!(
                                "controller {} - failed to read tag {}: {}",
                                self.opts.host,
                                tag.name(),
                                status.decode()
                            );
                            false
                        }
                    }
                })
                .collect();
            // stop loop if no pending tags
            if remaining.len() == 0 {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        //abort all remaining tags
        for tag in remaining.values() {
            error!(
                "controller {} - timeout to read tag {}",
                self.opts.host,
                tag.name()
            );
            let _ = tag.abort();
        }
        res.into_iter()
            .map(|x| (self.generate_key(x.name()), x))
            .collect()
    }

    /// you need to call `ensure_tag` to create tag if not exist
    pub fn post(&self, op: impl Operation + Send + 'static) {
        let messages = &mut *self.messages.lock();
        let key = op.id();
        //TODO: use policy
        messages.entry(key).or_insert(Box::new(op));
    }

    #[inline]
    pub(crate) fn generate_key(&self, tag_name: &str) -> String {
        format!("{},{}", self.opts.host, tag_name)
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let mut tags = self.tags.lock();
        tags.clear();
    }
}

#[derive(Debug)]
pub struct TagEntry {
    host: String,
    name: String,
    raw: RawTag,
    /// use atomicbool, unsafecell to remove mut self constrain
    ready: AtomicBool,
    created: Instant,
}

impl Deref for TagEntry {
    type Target = RawTag;
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
    pub fn new(opts: impl TagOptions) -> Result<Self> {
        Ok(Self {
            host: opts.host(),
            name: opts.name(),
            raw: RawTag::new(&opts.path(), 0)?,
            ready: AtomicBool::new(false),
            created: Instant::now(),
        })
    }

    #[inline]
    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn check_ready(&self, timeout: Duration) -> Result<bool> {
        if self.ready() {
            return Ok(true);
        }
        let status = self.raw.status();
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

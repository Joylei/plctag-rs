use crate::options::*;
use crate::tag::{ITag, Tag};
use crate::value::TagValue;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::Notify;

use std::collections::HashMap;

use futures::prelude::*;
use std::future::Future;
use std::result;
use std::sync::Arc;
use tokio::io;
use tokio::prelude::*;

pub struct Controller<'a> {
    opts: ControllerOptions,
    tags: Arc<Mutex<HashMap<String, Arc<Mutex<Box<dyn ITag + 'a>>>>>>,
    notify: Arc<Notify>,
}

impl<'a> Controller<'a> {
    pub(crate) fn new(opts: ControllerOptions) -> result::Result<Self, String> {
        opts.validate()?;
        Ok(Self {
            opts,
            tags: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        })
    }

    pub fn options(&self) -> &ControllerOptions {
        &self.opts
    }

    /// get or create tag
    pub async fn get_tag<T: TagValue + 'a>(
        &mut self,
        options: TagOptions,
    ) -> Arc<Mutex<Box<Tag<T>>>> {
        let map = &mut *self.tags.lock().await;
        let id = generate_id(&self.opts, &options);
        let res = if let Some(res) = map.get(&id) {
            res
        } else {
            let tag: Tag<T> = self.create(&options);
            let boxed: Arc<Mutex<Box<dyn ITag>>> = Arc::new(Mutex::new(Box::new(tag)));
            map.insert(id.to_string(), boxed);
            self.notify.notify();
            map.get(&id).unwrap()
        };
        let res = Arc::clone(res);
        downcast(res)
    }

    fn create<T: TagValue>(&self, options: &TagOptions) -> Tag<T> {
        Tag::create(&self.opts, options)
    }

    pub async fn remove_tag(&mut self, tag_id: &str) {
        let map = &mut *self.tags.lock().await;
        map.remove_entry(tag_id);
    }

    pub async fn tags(&self) -> Vec<Arc<Mutex<Box<dyn ITag + 'a>>>> {
        let map = &*self.tags.lock().await;
        map.values().map(|v| Arc::clone(&v)).collect()
    }
}

#[inline]
fn downcast<'a, T: TagValue + 'a>(src: Arc<Mutex<Box<dyn ITag + 'a>>>) -> Arc<Mutex<Box<Tag<T>>>> {
    let ptr = Arc::into_raw(src);
    let a_ptr = ptr as *const Mutex<Box<Tag<T>>>;
    unsafe { Arc::from_raw(a_ptr) }
}

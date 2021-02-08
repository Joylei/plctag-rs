use ffi::{plc_tag_register_callback, plc_tag_unregister_callback};
use parking_lot::Mutex;
use plctag_sys as ffi;
use std::{
    collections::{HashMap, HashSet},
    fmt, mem, panic,
    sync::{atomic::AtomicUsize, Weak},
};

pub(crate) use std::sync::Arc;

use crate::{status, Status};

pub type Event = i32;

pub struct Listener<'a, F: FnMut(Event, Status) + Send + 'static> {
    emitter: &'a Arc<EventEmitter>,
    manual: bool,
    handler: HandlerImpl<F>,
}

impl<'a, F: FnMut(Event, Status) + Send + 'static> Listener<'a, F> {
    #[inline(always)]
    pub(crate) fn new(emitter: &'a Arc<EventEmitter>, f: F) -> Self {
        Self {
            emitter,
            manual: true,
            handler: HandlerImpl::new(f),
        }
    }

    /// manually remove the listener, otherwise removed when drop
    #[inline(always)]
    pub fn manual(mut self, val: bool) -> Self {
        self.manual = val;
        self
    }

    /// listen for one events
    #[inline(always)]
    pub fn event(mut self, evt: Event) -> Self {
        self.handler.for_event(evt);
        self
    }

    /// listen for all events
    #[inline(always)]
    pub fn all(mut self) -> Self {
        self.handler.for_all();
        self
    }

    #[inline(always)]
    pub fn on(self) -> Remover {
        self.emitter.add(self.handler, self.manual)
    }
}

pub(crate) trait Handler: fmt::Debug {
    fn invoke(&mut self, evt: Event, status: Status);
}

struct HandlerImpl<F: FnMut(Event, Status) + Send + 'static> {
    interest: Option<HashSet<Event>>,
    once: bool,
    cb: F,
}

impl<F: FnMut(Event, Status) + Send + 'static> fmt::Debug for HandlerImpl<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("interest", &self.interest)
            .finish()
    }
}

impl<F: FnMut(Event, Status) + Send + 'static> HandlerImpl<F> {
    #[inline(always)]
    fn new(cb: F) -> Self {
        Self {
            interest: None,
            once: false,
            cb,
        }
    }

    #[inline(always)]
    fn for_event(&mut self, evt: Event) {
        if let Some(ref mut items) = self.interest {
            items.insert(evt);
        } else {
            let mut items = HashSet::new();
            items.insert(evt);
            self.interest = Some(items);
        }
    }
    #[inline(always)]
    fn for_all(&mut self) {
        self.interest = None;
    }

    #[inline(always)]
    fn interested(&self, evt: Event) -> bool {
        if let Some(ref items) = self.interest {
            items.iter().any(|v| *v == evt)
        } else {
            true
        }
    }
}

impl<F: FnMut(Event, Status) + Send + 'static> Handler for HandlerImpl<F> {
    #[inline(always)]
    fn invoke(&mut self, evt: Event, status: Status) {
        if self.interested(evt) {
            (self.cb)(evt, status)
        }
    }
}

/// event listener remover
pub struct Remover {
    id: usize,
    inner: Option<Weak<EventEmitter>>,
    manual: bool,
}

impl Remover {
    /// remove event handler
    #[inline(always)]
    pub fn off(self) {
        let mut v = self;
        v.remove_listener();
    }

    /// off() has been called or not
    #[inline(always)]
    fn called(&self) -> bool {
        self.inner.is_some()
    }

    fn remove_listener(&mut self) {
        self.inner
            .take()
            .map(|src| Weak::upgrade(&src))
            .flatten()
            .map(|src| src.remove(self.id));
    }
}

impl Drop for Remover {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.manual {
            self.remove_listener();
        }
    }
}

/// for testing purpose
static mut TAG_INSTALL: bool = true;

#[inline(always)]
fn can_install() -> bool {
    unsafe { TAG_INSTALL }
}

#[inline(always)]
fn set_install(val: bool) {
    if can_install() != val {
        unsafe { TAG_INSTALL = val };
    }
}

#[derive(Debug)]
pub(crate) struct EventEmitter {
    tag_id: i32,
    gen: AtomicUsize,
    map: Mutex<HashMap<usize, Box<dyn Handler + Send + 'static>>>,
}

impl EventEmitter {
    #[inline(always)]
    pub fn new(tag_id: i32) -> Arc<Self> {
        Arc::new(Self {
            tag_id,
            gen: AtomicUsize::new(0),
            map: Mutex::new(HashMap::new()),
        })
    }
}

impl EventEmitter {
    #[inline(always)]
    pub(crate) fn tag_id(&self) -> i32 {
        self.tag_id
    }

    #[inline(always)]
    fn remove(&self, id: usize) {
        let map = &mut *self.map.lock();
        map.remove(&id);
        if map.len() == 0 {
            EVENTS.remove(self.tag_id);
            if can_install() {
                let rc = unsafe { ffi::plc_tag_unregister_callback(self.tag_id()) };
                assert!(rc == status::PLCTAG_STATUS_OK);
            }
        }
    }

    #[inline(always)]
    fn gen_id(&self) -> usize {
        self.gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[inline(always)]
    pub(crate) fn listen<'a, F>(self: &'a Arc<Self>, f: F) -> Listener<'a, F>
    where
        F: FnMut(Event, Status) + Send + 'static,
    {
        Listener::new(&self, f)
    }

    #[inline(always)]
    pub(crate) fn add(
        self: &Arc<Self>,
        handler: impl Handler + Send + 'static,
        manual: bool,
    ) -> Remover {
        let id = self.gen_id();
        {
            let map = &mut *self.map.lock();
            map.insert(id, Box::new(handler));
            let install = map.len() == 1;
            //only install if len() changed from 0 to 1
            if install {
                EVENTS.add(self);
                if can_install() {
                    let rc =
                        unsafe { ffi::plc_tag_register_callback(self.tag_id(), Some(on_event)) };
                    assert!(rc == status::PLCTAG_STATUS_OK);
                }
            }
        }

        Remover {
            id,
            inner: Some(Arc::downgrade(self)),
            manual,
        }
    }

    #[inline(always)]
    fn emit(&self, event: Event, status: Status) {
        let map = &mut *self.map.lock();
        for h in map.values_mut() {
            h.invoke(event, status);
        }
    }
}

impl Drop for EventEmitter {
    #[inline(always)]
    fn drop(&mut self) {
        EVENTS.remove(self.tag_id);
    }
}

struct EventRegistry(Mutex<HashMap<i32, Weak<EventEmitter>>>);

impl EventRegistry {
    #[inline(always)]
    fn new() -> Self {
        EventRegistry(Mutex::new(HashMap::new()))
    }

    #[inline(always)]
    fn len(&self) -> usize {
        let map = &*self.0.lock();
        map.len()
    }

    #[inline]
    fn add(&self, item: &Arc<EventEmitter>) {
        let tag_id = item.tag_id;
        let item = Arc::downgrade(item);
        let map = &mut *self.0.lock();
        map.insert(tag_id, item);
    }

    #[inline(always)]
    fn remove(&self, tag_id: i32) {
        let map = &mut *self.0.lock();
        map.remove(&tag_id);
    }

    #[inline(always)]
    fn get(&self, tag_id: i32) -> Option<Arc<EventEmitter>> {
        let map = &*self.0.lock();
        map.get(&tag_id).map(|h| Weak::upgrade(h)).flatten()
    }

    #[inline]
    fn dispatch(&self, tag_id: i32, event: i32, status: i32) {
        if let Some(handle) = self.get(tag_id) {
            handle.emit(event, status.into());
        }
    }
}

lazy_static! {
    static ref EVENTS: EventRegistry = EventRegistry::new();
}

unsafe extern "C" fn on_event(tag_id: i32, event: i32, status: i32) {
    EVENTS.dispatch(tag_id, event, status)
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicI32, Ordering};

    use super::*;

    const EVENT_READ: Event = 0;
    const EVENT_WRITE: Event = 2;

    struct Holder {
        count: AtomicUsize,
        event: AtomicI32,
    }

    impl Holder {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
                event: AtomicI32::new(0),
            }
        }

        fn count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }

        fn event(&self) -> i32 {
            self.event.load(Ordering::SeqCst)
        }

        fn inc_count(&self) {
            self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        fn set_event(&self, val: i32) {
            self.event.store(val, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn test_on_off() {
        set_install(false);
        let holder = Arc::new(Holder::new());

        let emitter = EventEmitter::new(0);

        let holder1 = Arc::clone(&holder);
        let token1 = emitter
            .listen(move |e, s| {
                holder1.inc_count();
            })
            .event(EVENT_READ)
            .manual(true)
            .on();

        assert!(EVENTS.len() > 0);

        EVENTS.dispatch(0, EVENT_READ, 0);
        assert!(holder.count() == 1);

        token1.off();

        EVENTS.dispatch(0, EVENT_READ, 0);
        assert!(holder.count() == 1);
    }

    #[test]
    fn test_multiple() {
        set_install(false);

        let holder = Arc::new(Holder::new());

        let emitter = EventEmitter::new(0);

        let holder1 = Arc::clone(&holder);
        let token1 = emitter
            .listen(move |e, s| {
                holder1.set_event(e);
            })
            .event(EVENT_READ)
            .manual(true)
            .on();
        let holder2 = Arc::clone(&holder);
        let token2 = emitter
            .listen(move |e, s| {
                holder2.inc_count();
            })
            .all()
            .manual(true)
            .on();

        assert!(EVENTS.len() > 0);

        EVENTS.dispatch(0, EVENT_READ, 0);

        assert_eq!(holder.count(), 1);
        assert_eq!(holder.event(), EVENT_READ);

        EVENTS.dispatch(0, EVENT_WRITE, 0);

        assert_eq!(holder.count(), 2);
        assert_eq!(holder.event(), EVENT_READ);

        token1.off();

        EVENTS.dispatch(0, EVENT_READ, 0);
        assert_eq!(holder.count(), 3);
        assert_eq!(holder.event(), EVENT_READ);

        token2.off();

        EVENTS.dispatch(0, EVENT_READ, 0);
        assert_eq!(holder.count(), 3);
        assert_eq!(holder.event(), EVENT_READ);
    }
}

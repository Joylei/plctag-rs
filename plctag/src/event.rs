use fmt::Debug;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use plctag_sys as ffi;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
    ops::Deref,
    panic,
    sync::atomic::AtomicUsize,
    time::Instant,
};

use crate::{status, Status};

/// register tag for receiving events
pub(crate) fn register(tag_id: i32, reg_cb: bool) -> Token {
    let token = Token(Handle::new(tag_id));
    let emitter = EventEmitter::new(token.0.clone());
    EVENTS.add(emitter);

    if reg_cb {
        let rc = unsafe { ffi::plc_tag_register_callback(tag_id, Some(on_event)) };
        debug_assert!(rc == status::PLCTAG_STATUS_OK);
    }
    token
}

/// ensures that EventEmitter gets removed
#[derive(Debug)]
pub(crate) struct Token(Handle);

impl Drop for Token {
    fn drop(&mut self) {
        // let rc = unsafe { ffi::plc_tag_unregister_callback(self.id) };
        // debug_assert!(rc == status::PLCTAG_STATUS_OK);
        EVENTS.remove(self.id);
    }
}

impl Token {
    #[inline(always)]
    pub(crate) fn listen<'a, F>(&'a self, f: F) -> ListenerBuilder<'a, F>
    where
        F: FnMut(Event, Status) + Send + Sync + 'static,
    {
        ListenerBuilder::new(&self.0, f)
    }
}

impl Deref for Token {
    type Target = Handle;
    fn deref(&self) -> &Handle {
        &self.0
    }
}

/// event type
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Event {
    /// start reading
    ReadStarted,
    /// reading completed
    ReadCompleted,
    /// start writing
    WriteStarted,
    /// write completed
    WriteCompleted,
    /// connect/read/write aborted
    Aborted,
    /// tag destroyed
    Destroyed,
    /// other
    Other(i32),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Event::*;
        match self {
            ReadStarted => write!(f, "ReadStarted"),
            ReadCompleted => write!(f, "ReadCompleted"),
            WriteStarted => write!(f, "WriteStarted"),
            WriteCompleted => write!(f, "WriteCompleted"),
            Aborted => write!(f, "Aborted"),
            Destroyed => write!(f, "Destroyed"),
            Event::Other(v) => write!(f, "Other({})", v),
        }
    }
}

impl Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let v = (*self).into();
        state.write_i32(v);
    }
}

const PLCTAG_EVENT_READ_STARTED: i32 = ffi::PLCTAG_EVENT_READ_STARTED as i32;
const PLCTAG_EVENT_READ_COMPLETED: i32 = ffi::PLCTAG_EVENT_READ_COMPLETED as i32;
const PLCTAG_EVENT_WRITE_STARTED: i32 = ffi::PLCTAG_EVENT_WRITE_STARTED as i32;
const PLCTAG_EVENT_WRITE_COMPLETED: i32 = ffi::PLCTAG_EVENT_WRITE_COMPLETED as i32;
const PLCTAG_EVENT_ABORTED: i32 = ffi::PLCTAG_EVENT_ABORTED as i32;
const PLCTAG_EVENT_DESTROYED: i32 = ffi::PLCTAG_EVENT_DESTROYED as i32;

impl From<i32> for Event {
    fn from(evt: i32) -> Self {
        use Event::*;
        match evt {
            PLCTAG_EVENT_READ_STARTED => ReadStarted,
            PLCTAG_EVENT_READ_COMPLETED => ReadCompleted,
            PLCTAG_EVENT_WRITE_STARTED => WriteStarted,
            PLCTAG_EVENT_WRITE_COMPLETED => WriteCompleted,
            PLCTAG_EVENT_ABORTED => Aborted,
            PLCTAG_EVENT_DESTROYED => Destroyed,
            v => Other(v),
        }
    }
}

impl From<Event> for i32 {
    fn from(evt: Event) -> i32 {
        use Event::*;
        match evt {
            ReadStarted => PLCTAG_EVENT_READ_STARTED,
            ReadCompleted => PLCTAG_EVENT_READ_COMPLETED,
            WriteStarted => PLCTAG_EVENT_WRITE_STARTED,
            WriteCompleted => PLCTAG_EVENT_WRITE_COMPLETED,
            Aborted => PLCTAG_EVENT_ABORTED,
            Destroyed => PLCTAG_EVENT_DESTROYED,
            Event::Other(v) => v,
        }
    }
}

/// build an event listener.
///
/// By default, manual is true, and listen for all events.
///
/// # Examples
/// ## manually remove listener
/// must keep listener alive, otherwise you'll lose the chance to remove the listener from tag
/// ```rust,ignore
/// use plctag::event::Event;
/// let tag: RawTag = ...;
/// let listener = tag.listen(|evt, status|
/// {
///      println!("tag event: {}, status: {}", evt, status);   
/// })
/// .event(Event::ReadCompleted)
/// .manual(true)
/// .on();
///
/// //remove listener later by call Listener::off()
/// listener.off();
/// ```
/// ## auto remove listener
/// ```rust,ignore
/// use plctag::event::Event;
/// let tag: RawTag = ...;
/// {
///     let listener = tag.listen(|evt, status|
///     {
///          println!("tag event: {}, status: {}", evt, status);   
///     })
///     .event(Event::ReadCompleted)
///     .manual(false)
///     .on();
///     //do something with the tag
/// }
/// //here, listener removed <=
/// ```
pub struct ListenerBuilder<'a, F: FnMut(Event, Status) + Send + Sync + 'static> {
    handle: &'a Handle,
    manual: bool,
    handler: HandlerImpl<F>,
}

impl<'a, F: FnMut(Event, Status) + Send + Sync + 'static> ListenerBuilder<'a, F> {
    #[inline(always)]
    pub(crate) fn new(handle: &'a Handle, f: F) -> Self {
        Self {
            handle,
            manual: true,
            handler: HandlerImpl::new(f),
        }
    }

    /// manual = true, requires explicitly call [`Listener::off()`] to remove the callback;
    /// manual = false, the callback will be removed when [`Listener`] drops.
    #[inline(always)]
    pub fn manual(mut self, val: bool) -> Self {
        self.manual = val;
        self
    }

    /// listen for one event
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
    pub fn on(self) -> Listener {
        // ref key ensures token on the stack,
        // so does EventEmitter, it's safe to call unwrap()
        EVENTS
            .with(self.handle.id, |emitter| {
                emitter.add(self.handler, self.manual)
            })
            .unwrap()
    }
}

pub(crate) trait Handler: fmt::Debug {
    fn invoke(&mut self, evt: Event, status: Status);
}

struct HandlerImpl<F: FnMut(Event, Status) + Send + Sync + 'static> {
    interest: Option<HashSet<Event>>,
    cb: F,
}

impl<F: FnMut(Event, Status) + Send + Sync + 'static> fmt::Debug for HandlerImpl<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("interest", &self.interest)
            .finish()
    }
}

impl<F: FnMut(Event, Status) + Send + Sync + 'static> HandlerImpl<F> {
    #[inline(always)]
    fn new(cb: F) -> Self {
        Self { interest: None, cb }
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
            items.contains(&evt)
        } else {
            true
        }
    }
}

impl<F: FnMut(Event, Status) + Send + Sync + 'static> Handler for HandlerImpl<F> {
    #[inline(always)]
    fn invoke(&mut self, evt: Event, status: Status) {
        if self.interested(evt) {
            (self.cb)(evt, status)
        }
    }
}

/// event listener, see [`ListenerBuilder`] for usage.
pub struct Listener {
    id: usize,
    handle: Option<Handle>,
    manual: bool,
}

impl Listener {
    /// remove event handler
    #[inline(always)]
    pub fn off(self) {
        let mut v = self;
        v.remove_listener();
    }

    /// off() has been called or not
    #[inline(always)]
    fn called(&self) -> bool {
        self.handle.is_some()
    }

    #[inline(always)]
    fn remove_listener(&mut self) {
        self.handle.take().map(|handle| {
            EVENTS.with(handle.id, |emitter| {
                //lazily remove, tag id might be reused, check key
                if emitter.handle.eq(&handle) {
                    emitter.remove(self.id)
                }
            });
        });
    }
}

impl Drop for Listener {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.manual {
            self.remove_listener();
        }
    }
}

#[derive(Debug)]
pub(crate) struct EventEmitter {
    handle: Handle,
    gen: AtomicUsize,
    map: Mutex<HashMap<usize, Box<dyn Handler + Send + Sync + 'static>>>,
}

impl EventEmitter {
    #[inline(always)]
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            gen: AtomicUsize::new(0),
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl EventEmitter {
    #[inline(always)]
    pub(crate) fn tag_id(&self) -> i32 {
        self.handle.id().clone()
    }

    #[inline(always)]
    pub(crate) fn listen<'a, F>(&'a self, f: F) -> ListenerBuilder<'a, F>
    where
        F: Fn(Event, Status) + Send + Sync + 'static,
    {
        ListenerBuilder::new(&self.handle, f)
    }

    #[inline(always)]
    fn remove(&self, id: usize) {
        let map = &mut *self.map.lock();
        map.remove(&id);
    }

    #[inline(always)]
    fn next_id(&self) -> usize {
        self.gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    #[inline(always)]
    pub(crate) fn add(
        &self,
        handler: impl Handler + Send + Sync + 'static,
        manual: bool,
    ) -> Listener {
        let id = self.next_id();
        {
            let map = &mut *self.map.lock();
            map.insert(id, Box::new(handler));
        }

        Listener {
            id,
            handle: Some(self.handle.clone()),
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

/// resource handle
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Handle {
    id: i32,
    time: Instant,
}

impl Handle {
    pub(crate) fn new(id: i32) -> Self {
        Self {
            id,
            time: Instant::now(),
        }
    }

    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    pub(crate) fn time(&self) -> Instant {
        self.time
    }
}

struct EventRegistry(RwLock<HashMap<i32, EventEmitter>>);

impl EventRegistry {
    #[inline(always)]
    fn new() -> Self {
        EventRegistry(RwLock::new(HashMap::new()))
    }

    #[inline(always)]
    fn len(&self) -> usize {
        let map = &*self.0.read();
        map.len()
    }

    #[inline(always)]
    fn add(&self, item: EventEmitter) {
        let tag_id = item.tag_id();
        let map = &mut *self.0.write();
        map.insert(tag_id, item);
    }

    #[inline(always)]
    fn remove(&self, tag_id: i32) {
        let map = &mut *self.0.write();
        map.remove(&tag_id);
    }

    #[inline(always)]
    fn with<F: FnOnce(&EventEmitter) -> R, R>(&self, tag_id: i32, f: F) -> Option<R> {
        let map = &*self.0.read();
        map.get(&tag_id).map(f)
    }

    // #[inline(always)]
    // fn with_write<F: FnOnce(&mut EventEmitter) -> R, R>(&self, tag_id: i32, f: F) -> Option<R> {
    //     let map = &mut*self.0.write();
    //     map.get_mut(&tag_id).map(f)
    // }

    #[inline]
    fn dispatch(&self, tag_id: i32, event: i32, status: i32) -> bool {
        self.with(tag_id, |emitter| emitter.emit(event.into(), status.into()))
            .is_some()
    }
}

static EVENTS: Lazy<EventRegistry> = Lazy::new(|| EventRegistry::new());

unsafe extern "C" fn on_event(tag_id: i32, event: i32, status: i32) {
    EVENTS.dispatch(tag_id, event, status);
}

#[cfg(test)]
mod test {
    use std::sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    };

    use super::*;
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

        fn event(&self) -> Event {
            self.event.load(Ordering::SeqCst).into()
        }

        fn inc_count(&self) {
            self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        fn set_event(&self, val: Event) {
            self.event
                .store(val.into(), std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn test_multiple() {
        let holder = Arc::new(Holder::new());

        let token = register(1, false);

        let holder1 = Arc::clone(&holder);
        let handler1 = token
            .listen(move |e, s| {
                holder1.set_event(e);
            })
            .event(Event::ReadCompleted)
            .manual(true)
            .on();
        let holder2 = Arc::clone(&holder);
        let handler2 = token
            .listen(move |e, s| {
                holder2.inc_count();
            })
            .all()
            .manual(true)
            .on();

        EVENTS.dispatch(1, Event::ReadCompleted.into(), 0);

        assert_eq!(holder.count(), 1);
        assert_eq!(holder.event(), Event::ReadCompleted);

        EVENTS.dispatch(1, Event::WriteCompleted.into(), 0);
        assert_eq!(holder.count(), 2);

        handler1.off();

        EVENTS.dispatch(1, Event::ReadCompleted.into(), 0);
        assert_eq!(holder.count(), 3);
        assert_eq!(holder.event(), Event::ReadCompleted);

        handler2.off();

        EVENTS.dispatch(1, Event::ReadCompleted.into(), 0);
        assert_eq!(holder.count(), 3);
        assert_eq!(holder.event(), Event::ReadCompleted);
    }
}

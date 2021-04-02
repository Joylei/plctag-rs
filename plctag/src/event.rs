use fmt::Debug;
use once_cell::sync::Lazy;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
    hash::Hash,
};

use crate::{ffi, Status};

#[inline(always)]
pub(crate) fn listen<F>(tag_id: i32, f: F) -> Handler
where
    F: FnMut(i32, Event, Status) + Send + Sync + Clone + 'static,
{
    EVENTS.add_handler(tag_id, f)
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

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Handler {
    tag_id: i32,
    handler_id: u64,
}

impl Drop for Handler {
    fn drop(&mut self) {
        EVENTS.remove_handler(&self);
    }
}

trait Listener: dyn_clone::DynClone + Send + Sync {
    fn invoke(&mut self, tag_id: i32, evt: Event, status: Status);
}

#[derive(Clone)]
struct ListenerImpl<F: Clone> {
    f: F,
}

impl<F: Clone> ListenerImpl<F> {
    fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Listener for ListenerImpl<F>
where
    F: FnMut(i32, Event, Status) + Clone + Send + Sync + 'static,
{
    #[inline(always)]
    fn invoke(&mut self, tag_id: i32, evt: Event, status: Status) {
        (&mut self.f)(tag_id, evt, status);
    }
}

struct State {
    handlers: HashMap<i32, HashMap<u64, Box<dyn Listener + 'static>>>,
    next_handler_id: u64,
}

impl State {
    #[inline(always)]
    fn check_handler(&self, h: &Handler) -> bool {
        if let Some(handlers) = self.handlers.get(&h.tag_id) {
            handlers.contains_key(&h.handler_id)
        } else {
            false
        }
    }

    fn remove_handler(&mut self, h: &Handler) {
        let mut tag_removed = false;
        {
            if let Some(handlers) = self.handlers.get_mut(&h.tag_id) {
                handlers.remove(&h.handler_id);
                if handlers.len() == 0 {
                    tag_removed = true;
                }
            }
        }
        if tag_removed {
            self.handlers.remove(&h.tag_id);
            let rc = unsafe { ffi::plc_tag_unregister_callback(h.tag_id.clone()) };
            debug_assert_eq!(rc, ffi::PLCTAG_STATUS_OK as i32);
        }
    }
}

struct EventRegistry(RwLock<State>);

impl EventRegistry {
    #[inline(always)]
    fn new() -> Self {
        EventRegistry(RwLock::new(State {
            handlers: HashMap::new(),
            next_handler_id: 0,
        }))
    }

    #[inline(always)]
    fn remove_handler(&self, h: &Handler) {
        let reader = self.0.upgradable_read();
        if !reader.check_handler(h) {
            return;
        }
        let mut writer = RwLockUpgradableReadGuard::upgrade(reader);
        writer.remove_handler(&h);
    }

    fn add_handler<F>(&self, tag_id: i32, f: F) -> Handler
    where
        F: FnMut(i32, Event, Status) + Send + Sync + Clone + 'static,
    {
        let handler_id = {
            let mut writer = self.0.write();

            let handler_id = writer.next_handler_id;
            writer.next_handler_id += 1;
            let h: Box<dyn Listener + 'static> = Box::new(ListenerImpl::new(f));
            match writer.handlers.entry(tag_id) {
                Entry::Occupied(mut e) => {
                    e.get_mut().insert(handler_id.clone(), h);
                }
                Entry::Vacant(e) => {
                    let mut handlers = HashMap::new();
                    handlers.insert(handler_id.clone(), h);
                    e.insert(handlers);
                    let rc = unsafe { ffi::plc_tag_register_callback(tag_id, Some(on_event)) };
                    debug_assert_eq!(rc, ffi::PLCTAG_STATUS_OK as i32);
                }
            }
            handler_id
        };

        Handler { tag_id, handler_id }
    }

    fn dispatch(&self, tag_id: i32, event: i32, status: i32) -> bool {
        let res: Option<Vec<_>> = if event == PLCTAG_EVENT_DESTROYED {
            let reader = self.0.upgradable_read();
            if !reader.handlers.contains_key(&tag_id) {
                return false;
            }
            let mut writer = RwLockUpgradableReadGuard::upgrade(reader);
            let res = writer.handlers.remove(&tag_id);
            //let _ = ffi::plc_tag_unregister_callback(tag_id);
            drop(writer);
            res.map(|e| e.into_iter().map(|(_k, v)| v).collect::<Vec<_>>())
        } else {
            let reader = self.0.read();
            reader.handlers.get(&tag_id).map(|e| {
                e.values()
                    .map(|v| dyn_clone::clone_box(&**v))
                    .collect::<Vec<Box<dyn Listener>>>()
            })
        };
        if let Some(handlers) = res {
            let evt = event.into();
            let status = status.into();
            for mut h in handlers {
                h.invoke(tag_id, evt, status);
            }
            true
        } else {
            false
        }
    }
}

static EVENTS: Lazy<EventRegistry> = Lazy::new(|| EventRegistry::new());

unsafe extern "C" fn on_event(tag_id: i32, event: i32, status: i32) {
    EVENTS.dispatch(tag_id, event, status);
}

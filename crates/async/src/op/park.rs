// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use plctag_core::ffi;
use plctag_core::Status;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub enum Interest {
    Read,
    Write,
}

impl Interest {
    fn value(&self) -> u8 {
        match self {
            &Interest::Read => 1,
            &Interest::Write => 2,
        }
    }
}

#[derive(Debug)]
struct Parker {
    tx: Option<oneshot::Sender<Status>>,
    interest: Interest,
}

impl Parker {
    #[inline(always)]
    fn take(&mut self, interest: u8) -> Option<oneshot::Sender<Status>> {
        if self.interest.value() & interest == interest {
            self.tx.take()
        } else {
            None
        }
    }
}

#[inline(always)]
pub(crate) fn park(tag_id: i32, tx: oneshot::Sender<Status>, interest: Interest) {
    EVENTS.park(tag_id, tx, interest);
}

static EVENTS: Lazy<Registry> = Lazy::new(|| Registry(Mutex::new(Default::default())));

unsafe extern "C" fn on_event(tag_id: i32, event: i32, status: i32) {
    EVENTS.dispatch(tag_id, event, status);
}

struct Registry(Mutex<HashMap<i32, Parker>>);

impl Registry {
    #[inline]
    fn park(&self, tag_id: i32, tx: oneshot::Sender<Status>, interest: Interest) {
        let parker = Parker {
            tx: Some(tx),
            interest,
        };
        let mut should_register = false;

        let mut state = self.0.lock();
        match state.entry(tag_id) {
            Entry::Occupied(mut v) => {
                *v.get_mut() = parker;
            }
            Entry::Vacant(holder) => {
                should_register = true;
                holder.insert(parker);
            }
        }
        drop(state);

        if should_register {
            let rc = unsafe { ffi::plc_tag_register_callback(tag_id, Some(on_event)) };
            assert!(rc == 0);
        }
    }

    #[inline]
    fn dispatch(&self, tag_id: i32, event: i32, status: i32) {
        const PLCTAG_EVENT_READ_COMPLETED: i32 = ffi::PLCTAG_EVENT_READ_COMPLETED as i32;
        const PLCTAG_EVENT_WRITE_COMPLETED: i32 = ffi::PLCTAG_EVENT_WRITE_COMPLETED as i32;
        const PLCTAG_EVENT_DESTROYED: i32 = ffi::PLCTAG_EVENT_DESTROYED as i32;

        let interest = match event {
            PLCTAG_EVENT_READ_COMPLETED => Interest::Read.value(),
            PLCTAG_EVENT_WRITE_COMPLETED => Interest::Write.value(),
            PLCTAG_EVENT_DESTROYED => {
                let mut state = self.0.lock();
                state.remove(&tag_id);
                return;
            }
            _ => return,
        };

        let tx = {
            let mut state = self.0.lock();
            let item = state.get_mut(&tag_id);
            item.and_then(|v| v.take(interest))
        };
        tx.map(|tx| tx.send(status.into()));
    }
}

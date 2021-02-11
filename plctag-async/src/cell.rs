use std::{
    cell::UnsafeCell,
    hint,
    sync::{atomic::Ordering, Arc},
    task::Poll,
};
use std::{future::Future, sync::atomic::AtomicUsize, task::Waker};

const STATE_LOCKED: usize = 0b001;
const STATE_VALUE_SET: usize = 0b010;

pub(crate) struct SyncCell<T> {
    inner: Arc<State<T>>,
}

struct State<T> {
    state: AtomicUsize,
    value: UnsafeCell<Option<T>>,
    wakers: UnsafeCell<Option<Vec<Waker>>>,
}

impl<T> State<T> {
    fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            value: UnsafeCell::new(None),
            wakers: UnsafeCell::new(None),
        }
    }

    fn set(&self, val: T) -> bool {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state & STATE_VALUE_SET == STATE_VALUE_SET {
                return false;
            }
            if state & STATE_LOCKED == STATE_LOCKED {
                state = self.state.load(Ordering::Acquire);
                hint::spin_loop();
                continue;
            }
            //state == 0
            let old = self
                .state
                .compare_and_swap(state, state | STATE_LOCKED, Ordering::Release);
            if old != state {
                state = old;
                hint::spin_loop();
                continue;
            }
            //locked
            unsafe {
                *self.value.get() = Some(val);
                (&mut *self.wakers.get()).take().map(|mut items| loop {
                    if let Some(w) = items.pop() {
                        w.wake()
                    } else {
                        break;
                    }
                });
            }
            self.state.store(STATE_VALUE_SET, Ordering::Release);
            return true;
        }
    }

    fn is_set(&self) -> bool {
        let state = self.state.load(Ordering::Acquire);
        state & STATE_VALUE_SET == STATE_VALUE_SET
    }

    fn set_waker(&self, waker: &Waker) -> bool {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state & STATE_VALUE_SET == STATE_VALUE_SET {
                return false;
            }
            if state & STATE_LOCKED == STATE_LOCKED {
                state = self.state.load(Ordering::Acquire);
                hint::spin_loop();
                continue;
            }
            //state == 0
            let old = self
                .state
                .compare_and_swap(state, state | STATE_LOCKED, Ordering::Release);
            if old != state {
                state = old;
                hint::spin_loop();
                continue;
            }

            //lock
            let holder = unsafe { &mut *self.wakers.get() };
            if let Some(items) = holder {
                items.push(waker.clone());
            } else {
                *holder = Some(vec![waker.clone()]);
            }
            self.state.store(0, Ordering::Release);
            return true;
        }
    }

    fn get_unchecked(&self) -> Option<&T> {
        unsafe {
            let holder = &*self.value.get();
            if let Some(ref v) = holder {
                return Some(v);
            }
        }
        None
    }

    /// unsafe to get ref
    fn get(&self) -> Option<&T> {
        let state = self.state.load(Ordering::Relaxed);
        if state & STATE_VALUE_SET == STATE_VALUE_SET {
            return self.get_unchecked();
        }

        //more strict
        if self.is_set() {
            self.get_unchecked()
        } else {
            None
        }
    }

    /// unsafe to get ref
    fn get_mut(&self) -> Option<&mut T> {
        if self.is_set() {
            unsafe {
                let holder = &mut *self.value.get();
                if let Some(ref mut v) = holder {
                    return Some(v);
                }
            }
        }
        None
    }

    /// take value and reset state
    fn take(&self) -> Option<T> {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state == 0 {
                return None;
            }
            if state & STATE_LOCKED == STATE_LOCKED {
                state = self.state.load(Ordering::Acquire);
                hint::spin_loop();
                continue;
            }
            let old = self
                .state
                .compare_and_swap(state, state | STATE_LOCKED, Ordering::Release);
            if old != state {
                state = old;
                hint::spin_loop();
                continue;
            }

            //locked
            let val = unsafe {
                (&mut *self.wakers.get()).take();
                (&mut *self.value.get()).take()
            };
            self.state.store(0, Ordering::Release);
            return val;
        }
    }
}

unsafe impl<T: Send> Send for State<T> {}
unsafe impl<T: Sync> Sync for State<T> {}

impl<T> SyncCell<T> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(State::new()),
        }
    }

    pub(crate) fn set(&self, val: T) -> bool {
        self.inner.set(val)
    }

    fn is_set(&self) -> bool {
        self.inner.is_set()
    }

    pub(crate) fn get(&self) -> Option<&T> {
        self.inner.get()
    }

    fn get_mut(&self) -> Option<&mut T> {
        self.inner.get_mut()
    }

    fn take(self) -> Option<T> {
        self.inner.take()
    }
}

impl<T> Clone for SyncCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Default for SyncCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Future for SyncCell<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.is_set() {
            return Poll::Ready(());
        }
        if self.inner.set_waker(cx.waker()) {
            //check again
            if self.is_set() {
                return Poll::Ready(());
            }
        }
        Poll::Pending
    }
}

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
    #[inline(always)]
    fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            value: UnsafeCell::new(None),
            wakers: UnsafeCell::new(None),
        }
    }

    fn set(&self, val: T) -> bool {
        // only set value if value has not been set, otherwise waiting until lock is released
        let cur = 0;
        loop {
            let res =
                self.state
                    .compare_exchange(cur, STATE_LOCKED, Ordering::AcqRel, Ordering::Acquire);
            match res {
                Ok(_) => {
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
                Err(v) if v & STATE_VALUE_SET == STATE_VALUE_SET => return false,
                Err(_) => {
                    //another thread took lock
                    hint::spin_loop();
                    continue;
                }
            }
        }
    }
    #[inline(always)]
    fn is_set(&self) -> bool {
        let state = self.state.load(Ordering::Acquire);
        state & STATE_VALUE_SET == STATE_VALUE_SET
    }

    fn set_waker(&self, waker: &Waker) -> bool {
        // only set waker on initial state, otherwise waiting until lock is released
        let initial = 0;
        loop {
            let res = self.state.compare_exchange(
                initial,
                STATE_LOCKED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            match res {
                Ok(_) => {
                    //lock
                    let holder = unsafe { &mut *self.wakers.get() };
                    if let Some(items) = holder {
                        items.push(waker.clone());
                    } else {
                        *holder = Some(vec![waker.clone()]);
                    }
                    self.state.store(initial, Ordering::Release);
                    return true;
                }
                Err(v) if v & STATE_VALUE_SET == STATE_VALUE_SET => return false,
                Err(_) => {
                    //another thread took lock
                    hint::spin_loop();
                    continue;
                }
            }
        }
    }
    #[inline(always)]
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
}

unsafe impl<T: Send> Send for State<T> {}
unsafe impl<T: Sync> Sync for State<T> {}

impl<T> SyncCell<T> {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(State::new()),
        }
    }
    #[inline(always)]
    pub(crate) fn set(&self, val: T) -> bool {
        self.inner.set(val)
    }
    #[inline(always)]
    fn is_set(&self) -> bool {
        self.inner.is_set()
    }
    #[inline(always)]
    pub(crate) fn get(&self) -> Option<&T> {
        self.inner.get()
    }
}

impl<T> Clone for SyncCell<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Default for SyncCell<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Future for SyncCell<T> {
    type Output = ();
    #[inline(always)]
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

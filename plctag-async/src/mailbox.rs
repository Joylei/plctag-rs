use crate::{cell::SyncCell, Result};

use futures::{channel::mpsc, SinkExt, StreamExt};
use plctag::RawTag;

use std::{
    cell::UnsafeCell,
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::task::{self, JoinHandle};
use uuid::Uuid;

#[inline]
pub(crate) async fn create(mailbox: &Arc<Mailbox>, path: String) -> Token {
    let inner = Inner::new(path);
    let token = Token {
        id: inner.id,
        cell: inner.cell.clone(),
        mailbox: Arc::clone(mailbox),
    };
    mailbox.post(Message::Enqueue(inner)).await;
    token
}

pub(crate) struct Token {
    id: Uuid,
    cell: SyncCell<RawTag>,
    /// keep ref of mailbox, so background worker does not get dropped
    mailbox: Arc<Mailbox>,
}

impl Token {
    #[inline(always)]
    pub fn get(&self) -> plctag::Result<&RawTag> {
        match self.cell.get() {
            Some(v) => Ok(v),
            None => Err(plctag::Status::Pending),
        }
    }

    /// wait for ready
    #[inline(always)]
    pub async fn wait(&self) {
        let cell = self.cell.clone();
        cell.await
    }
}

impl Drop for Token {
    #[inline(always)]
    fn drop(&mut self) {
        self.mailbox.try_post(Message::Remove(self.id));
    }
}

struct State {
    retry_times: usize,
    begin_time: Instant,
    next_retry_time: Instant,
    /// used during status scanning;
    /// if initialized, value moved to cell
    tag: Option<RawTag>,
}

struct Inner {
    id: Uuid,
    path: String,
    state: UnsafeCell<State>,
    /// final value holder
    cell: SyncCell<RawTag>,
}

impl Inner {
    #[inline]
    fn new(path: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
            state: UnsafeCell::new(State {
                retry_times: 0,
                begin_time: Instant::now(),
                next_retry_time: Instant::now() - Duration::from_secs(1),
                tag: None,
            }),
            cell: SyncCell::new(),
        }
    }
    #[inline(always)]
    fn state(&self) -> &mut State {
        unsafe { &mut *self.state.get() }
    }

    fn check(&self) -> bool {
        let state = self.state();
        let status = match state.tag {
            Some(ref tag) => {
                let status = tag.status();
                if status.is_ok() {
                    self.set_result(state.tag.take().unwrap());
                    return true;
                }
                status
            }
            None => {
                if Instant::now() <= state.next_retry_time {
                    return false;
                }
                let res = RawTag::new(&self.path, 0);
                match res {
                    Ok(tag) => {
                        let status = tag.status();
                        if status.is_ok() {
                            self.set_result(tag);
                            return true;
                        }
                        if status.is_pending() {
                            state.tag = Some(tag); // for further checking
                        }
                        status
                    }
                    Err(status) => status,
                }
            }
        };

        if status.is_err() {
            trace!("tag[{}] initialization failed", self.id);
            self.on_error();
        }

        false
    }
    #[inline(always)]
    fn set_result(&self, tag: RawTag) {
        trace!("tag[{}] initialization ok: {:?}", self.id, &tag);
        let _ = self.cell.set(tag);
    }
    #[inline(always)]
    fn on_error(&self) {
        let state = self.state();
        state.retry_times = state.retry_times + 1;
        state.next_retry_time = Instant::now() + Duration::from_secs(1);
        state.tag = None;
        trace!("tag[{}] initialization will retry in 1 sec", self.id);
    }
}
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

enum Message {
    Enqueue(Inner),
    /// remove by key
    Remove(Uuid),
}

struct Processor {
    receiver: mpsc::UnboundedReceiver<Message>,
    pending: HashMap<Uuid, Arc<Inner>>,
}

impl Processor {
    #[inline(always)]
    fn new(receiver: mpsc::UnboundedReceiver<Message>) -> Self {
        Self {
            receiver,
            pending: HashMap::new(),
        }
    }
}

impl Processor {
    #[inline]
    async fn recv(&mut self) -> std::result::Result<Message, bool> {
        let v = if self.pending.len() == 0 {
            self.receiver.next().await
        } else {
            self.receiver.try_next().ok().flatten()
        };

        if let Some(v) = v {
            Ok(v)
        } else {
            Err(false)
        }
    }
    #[inline]
    async fn handle_message(&mut self, m: Message) {
        match m {
            Message::Enqueue(inner) => {
                trace!("tag[{}] initializing", &inner.id);
                self.pending.insert(inner.id, Arc::new(inner));
            }
            Message::Remove(id) => {
                self.pending.remove(&id);
                trace!("tag[{}] initialization cancelled", id);
            }
        }
    }

    async fn scan(&mut self) -> Result<()> {
        let mut ready_list = vec![];
        for item in self.pending.values() {
            let id = item.id;
            let done = {
                let item = Arc::clone(item);
                task::spawn_blocking(move || item.check()).await?
            };
            if done {
                ready_list.push(id);
            }
        }

        for key in ready_list {
            self.pending.remove(&key);
        }
        Ok(())
    }

    async fn run(&mut self) {
        loop {
            match self.recv().await {
                Ok(m) => self.handle_message(m).await,
                Err(true) => {}
                _ => break,
            };
            if self.pending.len() == 0 {
                task::yield_now().await;
            } else {
                if let Err(e) = self.scan().await {
                    trace!("MailboxProcessor - error: {}", e);
                }
            }
        }
        trace!("MailboxProcessor - loop end");
    }
}

unsafe impl Send for Processor {}

pub(crate) struct Mailbox {
    worker: JoinHandle<()>,
    sender: mpsc::UnboundedSender<Message>,
}

impl Mailbox {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();

        Self {
            worker: task::spawn(async {
                let mut p = Processor::new(receiver);
                p.run().await
            }),
            sender,
        }
    }
    #[inline(always)]
    fn try_post(&self, m: Message) {
        let mut sender = self.sender.clone();
        let _ = sender.start_send(m);
    }
    #[inline(always)]
    async fn post(&self, m: Message) {
        let mut sender = self.sender.clone();
        let _ = sender.send(m).await;
    }
}

impl Drop for Mailbox {
    fn drop(&mut self) {
        trace!("Mailbox - drop");
    }
}

use may::{
    coroutine as co,
    sync::{
        mpsc::{Receiver, Sender},
        SyncFlag,
    },
};
use once_cell::sync::OnceCell;

use std::{
    collections::HashMap,
    ops::Add,
    sync::{mpsc::TryRecvError, Arc},
    time::{Duration, Instant},
};
use uuid::Uuid;

//use crate::event;
use plctag::{RawTag, Result, Status};

#[inline]
pub(crate) fn create(mailbox: &Arc<Mailbox>, path: String) -> Token {
    let flag = Arc::new(SyncFlag::new());
    let inner = Inner::new(path, Arc::clone(&flag));
    let cell = Arc::clone(&inner.cell);
    let token = Token {
        id: inner.id.clone(),
        cell,
        mailbox: Arc::clone(mailbox),
        flag,
    };
    mailbox.post(Message::Enqueue(inner));
    token
}

enum Message {
    Enqueue(Inner),
    /// remove by key
    Remove(Uuid),
}

pub(crate) struct Mailbox {
    /// worker to init tag in background
    worker: co::JoinHandle<()>,
    sender: may::sync::mpsc::Sender<Message>,
}

impl Mailbox {
    pub fn new() -> Self {
        let (sender, receiver) = may::sync::mpsc::channel();
        let worker = {
            go!(|| {
                Processor::new(receiver).run();
                trace!("mailbox - exit loop");
            })
        };
        Self { worker, sender }
    }

    #[inline(always)]
    fn post(&self, m: Message) {
        let _ = self.sender.send(m);
    }
}

struct Processor {
    receiver: Receiver<Message>,
    pending: HashMap<Uuid, Inner>,
}

impl Processor {
    #[inline(always)]
    pub fn new(receiver: Receiver<Message>) -> Self {
        Self {
            receiver,
            pending: HashMap::new(),
        }
    }

    /// try receive
    #[inline]
    fn recv(&self) -> std::result::Result<Message, bool> {
        if self.pending.len() == 0 {
            self.receiver.recv().map_err(|_e| false)
        } else {
            //non blocking
            self.receiver.try_recv().map_err(|e| match e {
                TryRecvError::Empty => true,
                _ => false,
            })
        }
    }

    /// message loop & scanning tags
    pub fn run(&mut self) {
        loop {
            match self.recv() {
                Ok(m) => self.handle_message(m),
                Err(true) => {}
                _ => break,
            }

            if self.pending.len() == 0 {
                co::yield_now();
            } else {
                self.scan();
            }
        }
        trace!("MailboxProcessor - loop end");
    }

    /// process mssage
    #[inline(always)]
    fn handle_message(&mut self, m: Message) {
        match m {
            Message::Enqueue(inner) => {
                trace!("tag[{}] initializing", &inner.id);
                self.pending.insert(inner.id, inner);
            }
            Message::Remove(id) => {
                self.pending.remove(&id);
                trace!("tag[{}] initialization cancelled", id);
            }
        }
    }

    /// scan tag status
    #[inline]
    fn scan(&mut self) {
        let mut ready_list = vec![];

        //TODO: find best size
        const CHUNK_SIZE: usize = 1000;

        for (i, item) in self.pending.values_mut().enumerate() {
            if item.check() {
                ready_list.push(item.id);
            }

            if i % CHUNK_SIZE == 0 {
                co::yield_now();
            }
        }

        for key in ready_list {
            self.pending.remove(&key);
        }
    }
}

impl Drop for Mailbox {
    #[inline(always)]
    fn drop(&mut self) {
        trace!("Mailbox - drop");
        if !self.worker.is_done() {
            unsafe {
                self.worker.coroutine().cancel();
            }
            self.worker.wait();
        }
    }
}

/// inner state of tag creation;
/// once initialized, removed from Mailbox
struct Inner {
    id: Uuid,
    path: String,
    retry_times: usize,
    begin_time: Instant,
    next_retry_time: Instant,
    /// used during status scanning;
    /// if initialized, value moved to cell
    tag: Option<RawTag>,
    /// final value holder
    cell: Arc<OnceCell<RawTag>>,
    flag: Arc<SyncFlag>,
}

impl Inner {
    #[inline(always)]
    fn new(path: String, flag: Arc<SyncFlag>) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
            retry_times: 0,
            begin_time: Instant::now(),
            next_retry_time: Instant::now() - Duration::from_secs(1),
            tag: None,
            cell: Arc::new(Default::default()),
            flag,
        }
    }

    fn check(&mut self) -> bool {
        let status = match self.tag {
            Some(ref tag) => {
                let status = tag.status();
                if status.is_ok() {
                    match self.tag.take() {
                        Some(v) => {
                            self.set_result(v);
                            return true;
                        }
                        None => unreachable!(),
                    }
                }
                status
            }
            None => {
                if Instant::now() <= self.next_retry_time {
                    return false;
                }
                let res = RawTag::new(&self.path, 0);
                match res {
                    Ok(tag) => {
                        let status = tag.status();
                        if status.is_ok() {
                            self.set_result(tag);
                            return true;
                        } else if status.is_pending() {
                            self.tag = Some(tag); // for further checking
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
        trace!("tag[{}] initialization ok: {:?}", self.id, &tag,);
        if self.cell.set(tag).is_ok() {
            self.flag.fire();
        }
    }
    #[inline(always)]
    fn on_error(&mut self) {
        self.retry_times = self.retry_times + 1;
        self.next_retry_time = Instant::now().add(Duration::from_secs(1));
        self.tag = None;
        trace!("tag[{}] initialization will retry in 1 sec", self.id);
    }
}

#[derive(Clone)]
pub(crate) struct Token {
    id: Uuid,
    cell: Arc<OnceCell<RawTag>>,
    /// keep ref of mailbox, so background worker does not get dropped
    mailbox: Arc<Mailbox>,
    flag: Arc<SyncFlag>,
}

impl Token {
    #[inline(always)]
    pub fn get(&self) -> Result<&RawTag> {
        match self.cell.get() {
            Some(v) => Ok(v),
            None => Err(Status::Pending),
        }
    }
    #[inline(always)]
    pub fn wait(&self, timeout: Option<Duration>) -> bool {
        if self.cell.get().is_some() {
            return true;
        }
        if let Some(timeout) = timeout {
            self.flag.wait_timeout(timeout)
        } else {
            self.flag.wait();
            true
        }
    }
}

impl Drop for Token {
    #[inline(always)]
    fn drop(&mut self) {
        self.mailbox.post(Message::Remove(self.id))
    }
}

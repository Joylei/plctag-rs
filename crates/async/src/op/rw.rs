// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use super::park::{park, Interest};
use crate::Result;
use plctag_core::{RawTag, Status};
use tokio::sync::oneshot;

pub(crate) struct Operation<'a> {
    tag: &'a RawTag,
    interest: Interest,
    rx: Option<oneshot::Receiver<Status>>,
}

impl<'a> Operation<'a> {
    pub fn new(tag: &'a RawTag, interest: Interest) -> Self {
        Self {
            tag,
            interest,
            rx: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let tag_id = self.tag.id();
        let (tx, rx) = oneshot::channel();
        self.rx = Some(rx);
        park(tag_id, tx, self.interest);
        let mut status = match self.interest {
            Interest::Read => self.tag.read(0),
            Interest::Write => self.tag.write(0),
        };
        if status.is_pending() {
            match self.rx {
                Some(ref mut rx) => {
                    status = rx.await.unwrap();
                }
                None => unreachable!(),
            }
        }
        status.into_result()?;
        Ok(())
    }
}

impl Drop for Operation<'_> {
    fn drop(&mut self) {
        if self.rx.is_some() {
            let _ = self.tag.abort();
        }
    }
}

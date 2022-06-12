use std::any::Any;
use std::time::Duration;

use tokio::sync::mpsc::{Sender, UnboundedSender};
use tokio::time::sleep;

use crate::message::Message;

#[derive(Clone)]
enum SenderType {
    Unbounded(UnboundedSender<Message>),
    Bounded(Sender<Message>)
}

impl SenderType {
    pub(crate) fn send(&self, msg: Message) {
        match self {
            SenderType::Unbounded(tx) => {
                tx.send(msg);
            }
            SenderType::Bounded(tx) => {
                let tx = tx.clone();
                tokio::spawn(async move {
                    tx.send(msg).await;
                });
            }
        }
    }
}


/// Represents the address of an [Actor](crate::actor::Actor). Each [Actor](crate::actor::Actor)
/// has exactly one [Addr] through which other [Actor](crate::actor::Actor)'s can communicate with
/// it.
pub struct Addr {
    tx: SenderType
}

impl Addr {
    pub(crate) fn unbounded(tx: UnboundedSender<Message>) -> Self {
        Self {
            tx: SenderType::Unbounded(tx)
        }
    }

    pub(crate) fn bounded(tx: Sender<Message>) -> Self {
        Self {
            tx: SenderType::Bounded(tx)
        }
    }

    pub(crate) fn send(&self, msg: Message) {
        self.tx.send(msg);
    }

    fn send_with_delay(&self, msg: Message, delay: Duration) {
        let tx = self.tx.clone();

        tokio::spawn(async move {
            sleep(delay).await;
            tx.send(msg);
        });
    }

    /// Sends the given message to the [Actor](crate::actor::Actor) behind this [Addr] without
    /// specifying a reply_to address.
    pub fn tell<M: Any + Send>(&self, msg: M) {
        let msg = Message::without_sender(msg);
        self.send(msg);
    }

    /// Sends the given message to the [Actor](crate::actor::Actor) behind this [Addr] with
    /// a reply_to address.
    pub fn ask<M: Any + Send>(&self, msg: M, reply_to: Addr) {
        let msg = Message::with_sender(msg, reply_to);
        self.send(msg);
    }

    /// Sends the given message to the [Actor](crate::actor::Actor) behind this [Addr] after a
    /// specified delay without specifying a reply_to address.
    pub fn tell_delayed<M: Any + Send>(&self, msg: M, delay: Duration) {
        let msg = Message::without_sender(msg);
        self.send_with_delay(msg, delay);
    }

    /// Sends the given message to the [Actor](crate::actor::Actor) behind this [Addr] after a
    /// specified delay with a reply_to address.
    pub fn ask_delayed<M: Any + Send>(&self, msg: M, reply_to: Addr, delay: Duration) {
        let msg = Message::with_sender(msg, reply_to);
        self.send_with_delay(msg, delay);
    }
}

impl Clone for Addr {
    fn clone(&self) -> Self {
        Addr {
            tx: self.tx.clone()
        }
    }
}

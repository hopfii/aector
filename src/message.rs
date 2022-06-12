use std::any::{Any, TypeId};

use crate::address::Addr;

pub(crate) struct BroadcastMessage<M: Any + Clone + Send> {
    inner: M,
    addr: Option<Addr>
}

impl<M: Any + Clone + Send> BroadcastMessage<M> {
    pub(crate) fn without_sender(obj: M) -> Self {
        BroadcastMessage {
            inner: obj,
            addr: None
        }
    }

    pub(crate) fn with_sender(obj: M, sender: Addr) -> Self {
        BroadcastMessage {
            inner: obj,
            addr: Some(sender)
        }
    }

    pub(crate) fn get_message(&self) -> Message {
        let inner = self.inner.clone();
        match &self.addr {
            None => {
                Message::without_sender(inner)
            }
            Some(addr) => {
                Message::with_sender(inner, addr.clone())
            }
        }
    }
}



/// All types of messages which are sent from and to [Actor](crate::actor::Actor)'s are internally stored as [Message]. The
/// only requirements for a type to be qualified as a message is that it implements [Any] and [Send].
/// The message data is owned by this type and, if not explicitly stored by the receiving [Actor](crate::actor::Actor),
/// dropped after handling the message.
pub struct Message {
    inner: Box<dyn Any + Send>,
    pub(crate) sender: Option<Addr>
}

impl Message {
    pub(crate) fn with_sender<M: Any + Send>(obj: M, sender: Addr) -> Self {
        Self {
            inner: Box::new(obj),
            sender: Some(sender)
        }
    }

    pub(crate) fn without_sender<M: Any + Send>(obj: M) -> Self {
        Self {
            inner: Box::new(obj),
            sender: None
        }
    }

    pub(crate) fn instance_of<M: Any + Send>(&self) -> bool {
        self.inner.as_ref().type_id() == TypeId::of::<M>()
    }

    pub(crate) fn type_id(&self) -> TypeId {
        self.inner.as_ref().type_id()
    }

    pub(crate) fn downcast<M: Any + Send>(self) -> Box<M> {
        let inner = self.inner;

        let msg = inner.downcast::<M>();

        // unwrap here is on purpose - if this goes wrong something else has gone very wrong and the panic is ok
        msg.unwrap()
    }
}

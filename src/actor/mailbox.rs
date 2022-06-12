use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use crate::{Addr, Message};

enum Queue {
    Bounded(Receiver<Message>),
    Unbounded(UnboundedReceiver<Message>)
}

impl Queue {
    pub(crate) async fn recv(&mut self) -> Option<Message> {
        match self {
            Queue::Bounded(rx) => {
                rx.recv().await
            }
            Queue::Unbounded(rx) => {
                rx.recv().await
            }
        }
    }
}

pub(crate) struct Mailbox {
    queue: Queue,
    addr: Addr
}

impl Mailbox {

    pub(crate) fn bounded(buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        let queue = Queue::Bounded(rx);
        let addr = Addr::bounded(tx);
        Mailbox {
            queue,
            addr
        }
    }

    pub(crate) fn unbounded() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let queue = Queue::Unbounded(rx);
        let addr = Addr::unbounded(tx);
        Mailbox {
            queue,
            addr
        }
    }

    pub(crate) async fn recv(&mut self) -> Option<Message> {
        self.queue.recv().await
    }

    pub(crate) fn get_addr(&self) -> Addr {
        self.addr.clone()
    }

}
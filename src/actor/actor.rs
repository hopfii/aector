use std::error::Error;
use std::sync::Arc;
use crate::actor::actor_context::{ActorContext, ContextFlag};
use crate::actor::backup::Backup;
use crate::actor::mailbox::Mailbox;
use crate::actor_system::ActorSystem;
use crate::address::Addr;
use crate::behavior::Behavior;
use crate::message::Message;

/// ExitReason passed on to ActorSystem.
#[derive(Clone, Copy, Debug)]
pub enum ExitReason {
    Kill,
    Restart,
    Error
}

// static lifetime on S: no problem since actor has to be 'static anyways (i.e. contain no external refs)
// and state has to live at least as long as the actor, thus also 'static
/// This struct represents an Actor.
/// It consists of a mutable state S and a [Behavior], which specifies whether and how [Message]'s
/// of different types are to be handled. The only way to communicate with an [Actor] is through
/// it's [Addr].
pub struct Actor<S: Send + 'static> {
    state: S,
    behavior: Behavior<S>,
    mailbox: Mailbox,
    addr: Addr,
    context: ActorContext
}

/// Represents the capacity of the FIFO queue used for the mailbox of the actor.
pub enum MailboxType {
    /// Bounded queue where the given usize equals the maximal number of messages which can be kept
    /// in the mailbox. Messages which arrive after the mailbox has reached its capacity are silently dropped.
    Bounded(usize),
    /// Unbounded queue where the only upper limit of number of messages which can be stored is the
    /// available memory.
    Unbounded
}

impl<S: Send + 'static> Actor<S> {
    /// Creates an actor with the given initial state, behavior and the specified mailboxtype.
    pub fn new(state: S, behavior: Behavior<S>, mailbox_type: MailboxType) -> Self {
        let mailbox;
        match mailbox_type {
            MailboxType::Bounded(buffer_size) => {
                mailbox = Mailbox::bounded(buffer_size);
            }
            MailboxType::Unbounded => {
                mailbox = Mailbox::unbounded();
            }
        }

        let addr = mailbox.get_addr();

        let ctx = ActorContext::new(addr.clone());
        Self {
            state,
            behavior,
            mailbox: mailbox,
            addr: addr,
            context: ctx
        }
    }

    fn handle(&mut self, m: Message) -> Option<Box<dyn Error>> {
        // handle message
        let res = self.behavior.handle(m, &mut self.state, &mut self.context);

        match res {
            Ok(new_behavior) => {
                // if user-defined handler defines a new behavior set behavior of actor to this new behavior
                if let Some(new_behavior) = new_behavior {
                    self.behavior = new_behavior;
                }
                None
            }
            Err(err) => {
                Some(err)
            }
        }
    }

    fn on_start(&mut self) {
        if let Some(f) = self.behavior.on_start {
            f(&mut self.state, &mut self.context);
        }
    }

    fn on_error(&mut self) {
        if let Some(f) = self.behavior.on_error {
            f(&mut self.state, &mut self.context);
        }
    }

    fn on_kill(&mut self) {
        if let Some(f) = self.behavior.on_kill {
            f(&mut self.state, &mut self.context);
        }
    }

    fn on_restart(&mut self) {
        if let Some(f) = self.behavior.on_restart {
            f(&mut self.state, &mut self.context);
        }
    }


    pub(crate) async fn run(&mut self) -> ExitReason {
        self.on_start();
        loop {
            match self.context.flag {
                ContextFlag::Run => {
                    if let Some(msg) = self.mailbox.recv().await {
                        // run handler for message and check for error in closure
                        if let Some(_err) = self.handle(msg) {
                            self.on_error();
                            // propagate error up to actor_system for supervision strategy - we dont care what type of error occured
                            return ExitReason::Error;
                        }
                    }
                }
                ContextFlag::Kill => {
                    self.on_kill();
                    return ExitReason::Kill;
                },
                ContextFlag::Restart => {
                    self.on_restart();
                    return ExitReason::Restart;
                }
            }
        }
    }

    /// Returns the actors address.
    pub fn get_addr(&self) -> Addr {
        self.addr.clone()
    }


    pub(crate) fn set_actor_sys(&mut self, sys: Arc<ActorSystem>) {
        self.context.set_actor_sys(sys);
    }

    /// This function can be used for testing an [Actor]'s inner state.
    pub fn check_state(&self, check: fn(&S) -> bool) -> bool {
        check(&self.state)
    }

}

impl<S: Send + 'static + Clone> Actor<S> {

    pub(crate) fn create_backup(&self) -> Backup<S> {
        Backup::new(self.state.clone(), self.behavior.clone())
    }

    pub(crate) fn apply_backup(&mut self, backup: &Backup<S>) {
        self.state = backup.get_state();
        self.behavior = backup.get_behavior();
    }
}

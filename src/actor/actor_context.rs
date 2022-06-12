use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use crate::actor::actor::Actor;

use crate::actor_system::{ActorSystem, ActorSystemError};
use crate::address::Addr;
use crate::supervision::SupervisionStrategy;

#[derive(Clone, Copy)]
/// Represents the internal run state of an actor.
pub(crate) enum ContextFlag {
    Run,
    Kill,
    Restart
}

/// This struct represents the actors internal properties such as its address, the current run state
/// and holds a shared reference to its parent actor system for spawning new actors.
pub struct ActorContext {
    addr: Addr,
    pub(crate) flag: ContextFlag,
    sys: Option<Arc<ActorSystem>>
}

impl ActorContext {

    pub(crate) fn new(addr: Addr) -> Self {
        Self {
            addr,
            flag: ContextFlag::Run,
            sys: None
        }
    }

    /// Sets the internal reference to the parents ActorSystem
    pub(crate) fn set_actor_sys(&mut self, sys: Arc<ActorSystem>) {
        // this handler is called once the actor has been spawned on an actor_sys
        self.sys = Some(sys);
    }

    /// Spawns the given [Actor] on the [ActorSystem] of this [Actor].
    /// This function works identically to ActorSystem.spawn, but can be called from
    /// within an actors handler without reference to the ActorSystem.
    pub fn spawn<S: Send + 'static>(&mut self, actor: Actor<S>, name: String) -> Result<(), ActorSystemError> {
        match &self.sys {
            None => {
                // actor cant spawn other actors with this actor has not been spawned on any actor system yet
                Err(ActorSystemError::ActorNotSpawnedYet)
            }
            Some(sys) => {
                sys.spawn(actor, name)
            }
        }
    }

    /// Spawns the given actor on the actor system of this actor with the given supervision strategy.
    /// This function works identically to [ActorSystem#method.spawn_with_supervision], but can be called from
    /// within an actors handler without needing a reference to the [ActorSystem].
    pub fn spawn_with_supervision<S: Send + Clone>(self: &Arc<Self>, actor: Actor<S>, supervision_strategy: Box<dyn SupervisionStrategy<S> + Send>, name: String) -> Result<(), ActorSystemError> {
        match &self.sys {
            None => {
                // actor cant spawn other actors if this actor has not been spawned on any actor system yet
                Err(ActorSystemError::ActorNotSpawnedYet)
            }
            Some(sys) => {
                sys.spawn_with_supervision(actor, supervision_strategy, name)
            }
        }
    }

    /// Queries this actors actor system for another actor with the given name. Returns the [Addr]
    /// of the sought for actor if it exists.
    pub fn query(&self, name: &str) -> Option<Addr> {
        match &self.sys {
            None => {
                None
            }
            Some(sys) => {
                sys.query(name)
            }
        }
    }

    /// Stops the execution of the actor system and all associated actors.
    pub fn stop(&self) {
        match &self.sys {
            None => {}
            Some(sys) => {
                sys.stop();
            }
        }
    }

    /// Returns the [Addr] of this [Actor].
    pub fn get_addr(&self) -> Addr {
        self.addr.clone()
    }

    /// Kills current actor.
    pub fn kill(&mut self) {
        self.flag = ContextFlag::Kill;
    }

    /// Triggers a restart request. If the actors has been spawned with a supervision strategy, the
    /// actor will be restarted with its initial state and behavior. Otherwise, the actor will be killed.
    pub fn restart(&mut self) {
        self.flag = ContextFlag::Restart;
    }

    /// Runs the given function in an async task. This function does not block the handlers flow
    /// and may run/continue to run even after the handlers scope has been exited.
    pub fn run_async(&self, f: Box<dyn Fn() -> () + Send>) {
        tokio::spawn(async move {
            f();
        });
    }

    /// Runs the given function after a given delay. This function does not block the handlers flow
    /// and may run even after the handlers scope has been exited.
    pub fn run_delayed(&self, f: Box<dyn Fn() -> () + Send>, delay: Duration) {
        tokio::spawn(async move {
            sleep(delay).await;
            f();
        });
    }

    /// Sends given message to all [Actor]'s which are run on this [ActorSystem] without
    /// specifying a reply_to [Addr](crate::address::Addr).
    pub fn broadcast_tell<M: Send + Any + Clone>(&self, msg: M) -> Result<(), ActorSystemError> {
        match &self.sys {
            None => {
                Err(ActorSystemError::ActorNotSpawnedYet)
            }
            Some(sys) => {
                Ok(sys.broadcast_tell(msg))
            }
        }
    }

    /// Sends given message to all [Actor]'s which are run on this [ActorSystem] with a given reply_to [Addr](crate::address::Addr).
    pub fn broadcast_ask<M: Send + Any + Clone>(&self, msg: M, reply_to: Addr) -> Result<(), ActorSystemError> {
        match &self.sys {
            None => {
                Err(ActorSystemError::ActorNotSpawnedYet)
            }
            Some(sys) => {
                Ok(sys.broadcast_ask(msg, reply_to))
            }
        }
    }
}

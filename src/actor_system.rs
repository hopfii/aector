//! This module contains all data structures which are relevant for using [ActorSystem].

use std::any::Any;
use std::fmt::{Debug};
use std::sync::{Arc, Mutex};
use dashmap::DashMap;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{error, info, instrument};

use crate::actor::{Actor, ExitReason};
use crate::address::Addr;
use crate::message::BroadcastMessage;
use crate::supervision::{SuperVisionAction, SupervisionStrategy};
use crate::testing::TestActor;

/// The [ActorSystem] represents a collection of [Actor]'s which can communicate with each other. All
/// [Actor]'s are registered in the [ActorSystem] with a unique actor name and are executed by it.
/// # Example
///
/// ```
/// use aector::actor::Actor;
/// use aector::actor_system::ActorSystem;
/// use aector::behavior::BehaviorBuilder;
///
/// // the actor system has to be run on an async runtime. Tokio is used here as an example.
/// #[tokio::main]
/// fn main() {
///     // create new actor system
///     let actor_system = ActorSystem::new();
///     // create a simple actor with a String as it's state and an empty Behavior
///     let actor = Actor::new("some state".to_owned(), BehaviorBuilder::new().build());
///     // spawn the actor without supervision strategy
///     actor_system.spawn(actor, "my_actor1".to_owned());
///     // start the system
///     actor_system.start().await;
/// }
///
/// ```
pub struct ActorSystem {
    registry: DashMap<String, Addr>,
    join_handles: Mutex<Vec<JoinHandle<()>>>
}

#[derive(Error, Debug)]
/// This enum represents different errors which can occur when using [ActorSystem].
pub enum ActorSystemError {
    #[error("An actor with the same name already exists in the registry!")]
    ActorNameAlreadyInUse,
    #[error("This actor has not been spawned yet!")]
    ActorNotSpawnedYet
}

impl ActorSystem {

    #[instrument]
    /// Creates a new empty [ActorSystem].
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            registry: DashMap::new(),
            join_handles: Mutex::new(Vec::new())
        })
    }

    /// Spawns a given [Actor] without a [SupervisionStrategy]. On error this actor will just exit.
    #[instrument(skip(self, actor), fields(actor_name = %name))]
    pub fn spawn<S: Send>(self: &Arc<Self>, mut actor: Actor<S>, name: String) -> Result<(), ActorSystemError> {

        let name_backup = name.clone();

        // check if another actor with same name already exists in registry
        if self.registry.contains_key(&name) {
            error!("Actor with same name already exists in this actor system!");
            return Err(ActorSystemError::ActorNameAlreadyInUse);
        }
        // set reference in actor to actor_system
        actor.set_actor_sys(self.clone());
        self.registry.insert(name, actor.get_addr());

        // Arc handle for passing on into future for removing actor from registry before killing actor
        let sys_ref = self.clone();

        let run_handle = tokio::spawn(async move {
            let actor_exit_reason = actor.run().await;

            match actor_exit_reason {
                _ => {
                    info!("Actor without supervision died! Cleaning up resources and removing actor {} from system", &name_backup);
                    // remove actor from registry before exiting run loop
                    sys_ref.registry.remove(&name_backup);
                    return;
                }
            }
        });
        // add to join_handles for proper shutdown
        let mut join_h = self.join_handles.lock().unwrap();
        join_h.push(run_handle);

        Ok(())
    }

    /// Spawns a given [Actor] with a given [SupervisionStrategy] and a unique name. The [SupervisionStrategy]
    /// will be called on the exit of the actor and decide, given the [ExitReason](crate::actor::ExitReason), what to do next.
    /// The state S of the actor has to implement [Clone] such that the initial state of the actor can
    /// be cloned and reused when restarting the actor. An actors [Behavior](crate::behavior::Behavior)
    /// always implements [Clone] by default. The initial state and [Behavior](crate::behavior::Behavior) is
    /// stored in a [Backup](crate::actor::Backup).
    #[instrument(skip(self, actor, supervision_strategy), fields(actor_name = %name))]
    pub fn spawn_with_supervision<S: Send + Clone>(self: &Arc<Self>, mut actor: Actor<S>, mut supervision_strategy: Box<dyn SupervisionStrategy<S> + Send>, name: String) -> Result<(), ActorSystemError> {

        // check if another actor with same name already exists in registry
        if self.registry.contains_key(&name) {
            error!("Actor with same name already exists in this actor system!");
            return Err(ActorSystemError::ActorNameAlreadyInUse);
        }
        // set reference in actor to actor_system
        actor.set_actor_sys(self.clone());

        info!("Creating backup of actors initial state and behavior");
        // create backup of initial state and behavior
        let actor_backup = actor.create_backup();

        let name_backup = name.clone();
        self.registry.insert(name, actor.get_addr());

        // Arc handle for passing on into future for removing actor from registry before killing actor
        let sys_ref = self.clone();

        let join_handle = tokio::spawn(async move {
            loop {
                let actor_exit_reason = actor.run().await;
                info!("Actor exited run loop with reason: {:?}", actor_exit_reason);

                let supervision_action = supervision_strategy.apply(actor_exit_reason, &actor_backup, &mut actor);
                info!("Supervision action: {:?}", &supervision_action);
                match supervision_action {
                    SuperVisionAction::Exit => {
                        info!("Cleaning up resources and removing actor {} from system", &name_backup);
                        // remove actor from registry before exiting run loop
                        sys_ref.registry.remove(&name_backup);
                        return;
                    }
                    SuperVisionAction::Restart => {
                        info!("Trying to restart the actor with its initial state and behavior");
                        // just continue with infinite run loop
                    }
                    SuperVisionAction::RestartDelayed(delay) => {
                        info!("Trying to restart the actor with its initial state and behavior after a delay of {}ms", delay.as_millis());
                        // async wait before continuing with run loop
                        sleep(delay).await;
                    }
                }
            }
        });
        // add to join_handles for proper shutdown
        let mut join_h = self.join_handles.lock().unwrap();
        join_h.push(join_handle);
        Ok(())
    }

    /// Stops the execution of the actor system and all associated actors.
    pub fn stop(self: &Arc<Self>) {
        self.registry.clear();
        let mut join_h = self.join_handles.lock().unwrap();
        for jh in join_h.iter_mut() {
            jh.abort();
        }
    }

    /// Starts the actor system. Note that this function is async and thus has to be .await-ed for
    /// the actor system to start.
    #[instrument(skip_all)]
    pub async fn start(&self) {
        // note: this function is async since a "normal", non async loop would get optimized away in cargo run --release and cause havoc
        // this workaround (start().await) has the same effect but does not cause this problem

        // this just blocks forever such that the tokio runtime in main does not go out of scope and exit
        while self.registry.len() > 0 {

        }
    }

    /// Searches the [ActorSystem] for an [Actor] with the given name. If successful this function
    /// returns [Option::Some(Addr)](crate::address::Addr) of the sought for [Actor], otherwise [Option::None].
    pub fn query(self: &Arc<Self>, name: &str) -> Option<Addr> {
        match self.registry.get(name) {
            None => {
                None
            }
            Some(addr) => {
                Some(addr.clone())
            }
        }
    }

    /// Sends given message to all [Actor]'s which are run on this [ActorSystem] without
    /// specifying a reply_to [Addr](crate::address::Addr).
    pub fn broadcast_tell<M: Send + Any + Clone>(&self, msg: M) {
        let broadcast_msg = BroadcastMessage::without_sender(msg);

        for addr in self.registry.iter_mut() {
            addr.send(broadcast_msg.get_message());
        }
    }

    /// Sends given message to all [Actor]'s which are run on this [ActorSystem] with a given reply_to [Addr](crate::address::Addr).
    pub fn broadcast_ask<M: Send + Any + Clone>(&self, msg: M, reply_to: Addr) {
        let broadcast_msg = BroadcastMessage::with_sender(msg, reply_to);

        for addr in self.registry.iter_mut() {
            addr.send(broadcast_msg.get_message());
        }
    }

    /// Spawns a given [TestActor] without a [SupervisionStrategy]. This function is used
    /// to test Actors with the testing framework and returns True for a successful test and false
    /// for a not successful test. Note that the result has to be await-ed in the test function.
    pub async fn spawn_test<S: Send>(self: &Arc<Self>, mut actor:  Actor<TestActor<S>>) -> bool {

        let name = "test_actor".to_string();
        let name_backup = name.clone();

        // set reference in actor to actor_system
        actor.set_actor_sys(self.clone());
        self.registry.insert(name, actor.get_addr());

        // Arc handle for passing on into future for removing actor from registry before killing actor
        let sys_ref = self.clone();

        let test_result = tokio::spawn(async move {
            let actor_exit_reason = actor.run().await;

            match actor_exit_reason {
                ExitReason::Kill => {
                    info!("ActorTest {} passed successfully", &name_backup);
                    // remove actor from registry before exiting run loop
                    sys_ref.registry.remove(&name_backup);
                    return true;
                }
                ExitReason::Restart => {
                    return false;
                }
                ExitReason::Error => {
                    info!("ActorTest {} failed with error.", &name_backup);
                    // remove actor from registry before exiting run loop
                    sys_ref.registry.remove(&name_backup);
                    return false;
                }
            }
        }).await;

        match test_result {
            Ok(res) => {
                return res;
            }
            Err(_) => {
                return false;
            }
        }
    }
}




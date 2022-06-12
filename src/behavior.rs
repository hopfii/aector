//! A behavior is used to describe how an actor behaves on specific system events and how and which
//! type of messages can be handled.

use std::any::{Any, TypeId};
use std::collections::{HashMap};
use std::error::Error;
use std::panic;
use std::sync::Arc;

use crate::actor::ActorContext;
use crate::address::Addr;
use crate::message::Message;

pub enum ActorManageMessage {
    Kill,
    Restart
}


pub type BehaviorAction<S: Send + 'static> = Result<Option<Behavior<S>>, Box<dyn Error>>;

/// Message handlers as stored internally. This type wraps user-defined handlers into a closure which
/// automatically does the downcasting.
/// Arc is used here instead of e.g. Box since HandlerFn has to be cloneable in order to create backups of the initial behavior of an actor for supervision.
type HandlerFn<S: Send + 'static> = Arc<dyn Fn(Message, &mut S, &mut ActorContext) -> BehaviorAction<S> + Send + Sync>;

/// Message handler as defined by user when working with BehaviorBuilder for ask (i.e. with passing Addr of sender of message)
type UserDefinedAskHandlerFn<M: Any + Send, S: Send + 'static> = fn(M, &mut S, Addr, &mut ActorContext) -> BehaviorAction<S>;
/// Message handler as defined by user when working with BehaviorBuilder for tell (i.e. without passing Addr of sender of message)
type UserDefinedTellHandlerFn<M: Any + Send, S: Send + 'static> = fn(M, &mut S, &mut ActorContext) -> BehaviorAction<S>;

/// Type of closures which are run by the actor without any message such as on_start, on_error, ..
type PlainActorAction<S: Send + 'static> = fn(&mut S, &mut ActorContext) -> ();

/// Message handler as defined by user when working with BehaviorBuilder for tell (i.e. without passing Addr of sender of message), but with taking a closure instead of a function pointer
type UserDefinedTellHandlerClosure<M: Any + Send, S: Send + 'static> = Box<dyn Fn(M, &mut S, &mut ActorContext) -> BehaviorAction<S> + Send + Sync>;
/// Message handler as defined by user when working with BehaviorBuilder for ask (i.e. with passing Addr of sender of message), but with taking a closure instead of a function pointer
type UserDefinedAskHandlerClosure<M: Any + Send, S: Send + 'static> = Box<dyn Fn(M, &mut S, Addr, &mut ActorContext) -> BehaviorAction<S> + Send + Sync>;


/// This struct is used to build a [Behavior].
pub struct BehaviorBuilder<S: Send + 'static> {
    on_ask_handler: HashMap<TypeId, HandlerFn<S>>,
    on_tell_handler: HashMap<TypeId, HandlerFn<S>>,
    on_start: Option<PlainActorAction<S>>,
    on_kill: Option<PlainActorAction<S>>,
    on_error: Option<PlainActorAction<S>>,
    on_restart: Option<PlainActorAction<S>>,
}

impl<S: Send + 'static> BehaviorBuilder<S> {
    /// Creates an empty BehaviorBuilder.
    pub fn new() -> Self {
        Self {
            on_ask_handler: HashMap::new(),
            on_tell_handler: HashMap::new(),
            on_start: None,
            on_kill: None,
            on_error: None,
            on_restart: None
        }
    }

    /// Defines handler for messages of type M for which the Addr of the sender has been passed on to the receiver.
    /// Only one ask_handler per message type can be defined per actor.
    pub fn on_ask<M: Any + Send>(mut self, h: UserDefinedAskHandlerFn<M, S>) -> Self {
        let h_wrapper = move |msg: Message, state: &mut S, ctx: &mut ActorContext| -> BehaviorAction<S>{

            // downcasting generic message into concrete type
            if msg.instance_of::<M>() {
                // checking if Addr of sender exists, otherwise calling ask is invalid!
                match &msg.sender {
                    Some(tx) => {
                        let sender = tx.clone();
                        let m = msg.downcast::<M>();
                        // passing downcasted message and sender addr on to user defined handler
                        h(*m, state, sender, ctx)
                    },
                    None => {
                        // ignore invalid usage of API - actor should not bother!
                        println!("Sent message without a sender to on_ask, response not possible!");
                        Ok(None)
                    }
                }
            } else {
                // this case should never occur, but if it does something has gone really wrong
                panic!("Invalid downcasting operation!")
            }
        };

        // check for duplicate handlers for same message type
        if self.on_ask_handler.contains_key(&TypeId::of::<M>()) {
            panic!("Ask handler for {} has already been defined on this behavior! Cannot define more than one ask handler per message type per actor!", std::any::type_name::<M>());
        } else {
            // store handler associated with type
            self.on_ask_handler.insert(TypeId::of::<M>(), Arc::new(h_wrapper));
            self
        }
    }


    /// Defines handler for messages of type M for which no Addr of the sender has been passed on to the receiver.
    /// Only one tell_handler per message type can be defined per actor.
    pub fn on_tell<M: Any + Send>(mut self, h: UserDefinedTellHandlerFn<M, S>) -> Self {
        let h_wrapper = move |msg: Message, state: &mut S, ctx: &mut ActorContext| -> BehaviorAction<S> {

            // downcasting generic message into concrete type
            if msg.instance_of::<M>() {
                // note: m.sender is totally ignored here i.e. can be Some(tx) or None
                let m = msg.downcast::<M>();
                // passing downcasted message on to user defined handler
                h(*m, state, ctx)
            } else {
                // this case should never occur, but if it does something has gone really wrong
                panic!("Invalid downcasting operation!")
            }
        };

        // check for duplicate handlers for same message type
        if self.on_tell_handler.contains_key(&TypeId::of::<M>()) {
            panic!("Tell handler for {} has already been defined on this behavior! Cannot define more than one tell handler per message type per actor!", std::any::type_name::<M>());
        } else {
            // store handler associated with type
            self.on_tell_handler.insert(TypeId::of::<M>(), Arc::new(h_wrapper));
            self
        }
    }

    /// This function defines the action an actor executes on its startup. This function is also called
    /// when an actor is restarted either after requesting it using [ActorContext.restart()](crate::actor::ActorContext#method.restart)
    /// or because of a restart caused by a [SupervisionStrategy](crate::supervision::SupervisionStrategy).
    pub fn on_start(mut self, action: PlainActorAction<S>) -> Self {
        if let Some(_) = self.on_start {
            panic!("Cannot define more than one on_start methods for same actor!");
        } else {
            self.on_start = Some(action);
            self
        }
    }

    /// This function defines the action an actor executes when it is killed by calling the
    /// [ActorContext.kill()](crate::actor::ActorContext#method.kill) function.
    pub fn on_kill(mut self, action: PlainActorAction<S>) -> Self {
        if let Some(_) = self.on_kill {
            panic!("Cannot define more than one on_kill methods for same actor!");
        } else {
            self.on_kill = Some(action);
            self
        }
    }

    /// This function defines the action an actor executes when it is killed cause of an error
    /// of any type occuring in an ask or tell handler.
    pub fn on_error(mut self, action: PlainActorAction<S>) -> Self {
        if let Some(_) = self.on_error {
            panic!("Cannot define more than one on_error methods for same actor!");
        } else {
            self.on_error = Some(action);
            self
        }
    }

    /// This function defines the action an actor executes after a restart has been requested using [ActorContext.restart()](crate::actor::ActorContext#method.restart)
    /// before the actor is restarted. This function is also called if a a [SupervisionStrategy](crate::supervision::SupervisionStrategy)
    /// decies to restart an actor.
    pub fn on_restart(mut self, action: PlainActorAction<S>) -> Self {
        if let Some(_) = self.on_restart {
            panic!("Cannot define more than one on_restart methods for same actor!");
        } else {
            self.on_restart = Some(action);
            self
        }
    }

    /// Enables the default handler for StateCheckMessage. This has to be called for all actors
    /// which are to be tested using the [crate::testing] module.
    pub fn enable_state_checks(self) -> Self {
        self.on_ask::<StateCheckMessage<S>>(|msg, state, reply_to, _ctx| -> BehaviorAction<S> {
            match msg {
                StateCheckMessage::Check(check_fn) => {
                    let res = check_fn(state);
                    reply_to.tell(StateCheckMessage::<S>::Result(res));
                }
                _ => {}
            }

            Behavior::keep()
        })
    }


    /// Defines handler for messages of type M for which no Addr of the sender has been passed on to the receiver.
    /// Only one tell_handler per message type can be defined per actor. This special function
    /// is used since the testing framework requires closures to be passed in which is not possible
    /// with bare function pointers.
    pub fn on_tell_closure<M: Any + Send>(mut self, h: UserDefinedTellHandlerClosure<M, S>) -> Self {
        let h_wrapper = move |msg: Message, state: &mut S, ctx: &mut ActorContext| -> BehaviorAction<S> {

            // downcasting generic message into concrete type
            if msg.instance_of::<M>() {
                // note: m.sender is totally ignored here i.e. can be Some(tx) or None
                let m = msg.downcast::<M>();
                // passing downcasted message on to user defined handler
                h(*m, state, ctx)
            } else {
                // this case should never occur, but if it does something has gone really wrong
                panic!("Invalid downcasting operation!")
            }
        };

        // check for duplicate handlers for same message type
        if self.on_tell_handler.contains_key(&TypeId::of::<M>()) {
            panic!("Tell handler for {} has already been defined on this behavior! Cannot define more than one tell handler per message type per actor!", std::any::type_name::<M>());
        } else {
            // store handler associated with type
            self.on_tell_handler.insert(TypeId::of::<M>(), Arc::new(h_wrapper));
            self
        }
    }

    /// Defines handler for messages of type M for which the Addr of the sender has been passed on to the receiver.
    /// Only one ask_handler per message type can be defined per actor.
    pub fn on_ask_closure<M: Any + Send>(mut self, h: UserDefinedAskHandlerClosure<M, S>) -> Self {
        let h_wrapper = move |msg: Message, state: &mut S, ctx: &mut ActorContext| -> BehaviorAction<S>{

            // downcasting generic message into concrete type
            if msg.instance_of::<M>() {
                // checking if Addr of sender exists, otherwise calling ask is invalid!
                match &msg.sender {
                    Some(tx) => {
                        let sender = tx.clone();
                        let m = msg.downcast::<M>();
                        // passing downcasted message and sender addr on to user defined handler
                        h(*m, state, sender, ctx)
                    },
                    None => {
                        // ignore invalid usage of API - actor should not bother
                        Ok(None)
                    }
                }
            } else {
                // this case should never occur, but if it does something has gone really wrong
                panic!("Invalid downcasting operation!")
            }
        };

        // check for duplicate handlers for same message type
        if self.on_ask_handler.contains_key(&TypeId::of::<M>()) {
            panic!("Ask handler for {} has already been defined on this behavior! Cannot define more than one ask handler per message type per actor!", std::any::type_name::<M>());
        } else {
            // store handler associated with type
            self.on_ask_handler.insert(TypeId::of::<M>(), Arc::new(h_wrapper));
            self
        }
    }

    pub(crate) fn has_tell_handler(&self, type_id: TypeId) -> bool {
        self.on_tell_handler.contains_key(&type_id)
    }

    pub(crate) fn has_ask_handler(&self, type_id: TypeId) -> bool {
        self.on_ask_handler.contains_key(&type_id)
    }


    /// Consumes the builder and returns a [Behavior].
    pub fn build(self) -> Behavior<S> {
        let b = self.on_tell::<ActorManageMessage>(|msg, _state, ctx| -> BehaviorAction<S> {

            match msg {
                ActorManageMessage::Kill => {
                    ctx.kill()
                },
                ActorManageMessage::Restart => {
                    ctx.restart()
                }
            }

            Behavior::keep()
        });

        Behavior {
            on_ask_handler: b.on_ask_handler,
            on_tell_handler: b.on_tell_handler,
            on_start: b.on_start,
            on_kill: b.on_kill,
            on_error: b.on_error,
            on_restart: b.on_restart
        }
    }
}

pub enum StateCheckMessage<S> {
    Check(fn(&S) -> bool),
    Result(bool)
}

#[derive(Clone)]
/// This struct defines the behavior of an actor. A behavior is defined by it's actions which
/// are executed under special circumstances (e.g. on start, on error, etc.) but also how messages
/// of different types and different requests (ask / tell) are handled. In order to build a [Behavior]
/// see [BehaviorBuilder]
pub struct Behavior<S: Send + 'static> {
    pub(crate) on_ask_handler: HashMap<TypeId, HandlerFn<S>>,
    pub(crate) on_tell_handler: HashMap<TypeId, HandlerFn<S>>,
    pub(crate) on_start: Option<PlainActorAction<S>>,
    pub(crate) on_kill: Option<PlainActorAction<S>>,
    pub(crate) on_error: Option<PlainActorAction<S>>,
    pub(crate) on_restart: Option<PlainActorAction<S>>,
}

impl<S: Send> Behavior<S> {

    /// This function indicates that the actor should keep its current behavior at the end of a
    /// handler scope.
    pub fn keep() -> BehaviorAction<S> {
        Ok(None)
    }

    /// This function indicates that the actor should change its current behavior at the end of a
    /// handler scope to the given new behavior.
    pub fn change(new_behavior: Behavior<S>) -> BehaviorAction<S> {
        Ok(Some(new_behavior))
    }
}

impl<S: Send + 'static> Behavior<S> {
    pub(crate) fn handle(&mut self, msg: Message, state: &mut S, ctx: &mut ActorContext) -> BehaviorAction<S> {
        // if message contains sender: assume on_ask handler, otherwise on_tell handler
        match &msg.sender {
            Some(_) => {
                // get on_ask handler
                match self.on_ask_handler.get(&msg.type_id()) {
                    Some(f) => {
                        f(msg, state, ctx)
                    },
                    None => {
                        // unsupported message types are just dropped silently
                        // println!("Message type not supported!");
                        Ok(None)
                    }
                }
            },
            None => {
                // get on_tell handler
                match self.on_tell_handler.get(&msg.type_id()) {
                    Some(f) => {
                        f(msg, state, ctx)
                    },
                    None => {
                        // unsupported message types are just dropped silently
                        // println!("Message type not supported!");
                        Ok(None)
                    }
                }
            }
        }
    }
}






use std::any::{Any, TypeId};
use std::collections::{VecDeque};
use std::fmt::{Debug, Formatter};
use crate::actor::{Actor, MailboxType};
use crate::{Addr, Message};
use crate::behavior::{Behavior, BehaviorBuilder, BehaviorAction, StateCheckMessage};
use crate::testing::actor_test::ResponseDyn::Check;
use crate::testing::actor_test::TestActorState::{PendingResponse, Ready};
use thiserror::Error;


#[derive(Error, Debug)]
enum ActorTestError {
    #[error("Invalid message order")]
    InvalidMessageOrder,
    #[error("Given criteria not fulfilled")]
    CriteriaNotMet,
    #[error("State check failed")]
    StateCheckFailed
}

/// This type represents an expected message response.
pub enum Response<M: Any + Send> {
    Ask(fn(M) -> bool),
    Tell(fn(M) -> bool),
    Check
}

impl<M> From<Response<M>> for ResponseDyn
where
    M: Any + Send
{
    fn from(res_t: Response<M>) -> Self {
        let type_id = TypeId::of::<M>();
        match res_t {
            Response::Ask(criteria) => {
                let crit_wrapper = ResponseDyn::wrap(criteria);
                ResponseDyn::Ask(type_id, crit_wrapper)
            }
            Response::Tell(criteria) => {
                let crit_wrapper = ResponseDyn::wrap(criteria);
                ResponseDyn::Tell(type_id, crit_wrapper)
            }
            Response::Check => {
                ResponseDyn::Check
            }
        }
    }
}

/// Dynamically typed responses. Only used to store responses internally (since generics cant be
/// directly stored in Vec)
enum ResponseDyn {
    Ask(TypeId, Box<dyn Fn(Message) -> bool + Send>),
    Tell(TypeId, Box<dyn Fn(Message) -> bool + Send>),
    Check
}

impl ResponseDyn {
    /// Wraps a given type and criteria into a dynamically typed enum
    pub fn tell<M: Any + Send>(criteria: fn(M) -> bool) -> ResponseDyn {
        let crit_wrapped = Self::wrap(criteria);
        ResponseDyn::Tell(TypeId::of::<M>(), crit_wrapped)
    }

    /// Wraps the given, generically typed closure into a dynamically typed, boxed closure.
    fn wrap<M: Any + Send>(criteria: fn(M) -> bool) -> Box<dyn Fn(Message) -> bool + Send> {
        let crit_wrapper = Box::new(move |msg: Message| -> bool {
            // downcasting generic message into concrete type
            if msg.instance_of::<M>() {
                // note: m.sender is totally ignored here i.e. can be Some(tx) or None
                let m = msg.downcast::<M>();
                // passing downcasted message on to user defined handler
                criteria(*m)
            } else {
                // this case should never occur, but if it does something has gone really wrong
                panic!("Invalid downcasting operation!")
            }
        });
        return crit_wrapper;
    }
}

/// Represents the state of the FSM of the testing actor.
enum TestActorState {
    Ready,
    PendingResponse(ResponseDyn)
}

/// Represents test-tasks defined by the user.
enum TestTask<S> {
    Tell(Message, u32),
    Ask(Message, ResponseDyn, u32),
    Check(fn(&S) -> bool, u32),
    Expect(ResponseDyn, u32),
    Exit
}

impl<S> Debug for TestTask<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TestTask::Tell(_, nr) => {
                write!(f, "Tell #{}", nr)
            }
            TestTask::Ask(_, _, nr) => {
                write!(f, "Ask #{}", nr)
            }
            TestTask::Check(_, nr) => {
                write!(f, "Check #{}", nr)
            }
            TestTask::Expect(_, nr) => {
                write!(f, "Expect #{}", nr)
            }
            TestTask::Exit => {
                write!(f, "Exit")
            }
        }

    }
}

/// This struct is used to store the testing state of an [TestActor]
pub struct TestActor<S> {
    addr: Addr,
    tasks: VecDeque<TestTask<S>>,
    test_state: TestActorState
}

/// Message used to reschedule messages to [TestActor].
enum TestActorMessage {
    RunNext
}

impl<S: Send + 'static> TestActor<S> {

    /// Returns blanket behavior for [TestActor]. This performs the task work loop
    fn get_blanket_behavior() -> BehaviorBuilder<TestActor<S>> {
        BehaviorBuilder::new()
            .on_start(|_state: &mut TestActor<S>, ctx| {
                // trigger self-loop for going through tasks
                ctx.get_addr().tell(TestActorMessage::RunNext);
            })
            .on_tell::<StateCheckMessage<S>>(|msg, state, ctx| -> BehaviorAction<TestActor<S>> {
                match &state.test_state {
                    Ready => {
                        // panic!("Did not expect a check result in the current state");
                        return Err(Box::new(ActorTestError::InvalidMessageOrder));
                    }
                    PendingResponse(resp) => {
                        match resp {
                            Check => {
                                // this handler is triggered if a result from last check_state query is received
                                match msg {
                                    StateCheckMessage::Check(_) => {
                                        // this message is only sent to the actor to be tested and should never come back
                                    }
                                    StateCheckMessage::Result(check_result) => {
                                        state.test_state = Ready;
                                        if check_result == false {
                                            // panic!("state check failed!");
                                            return Err(Box::new(ActorTestError::StateCheckFailed));
                                        }
                                    }
                                }

                                // continue working on tasks
                                ctx.get_addr().tell(TestActorMessage::RunNext);
                            }
                            _ => {
                                // panic!("Did not expect a check result in the current state");
                                return Err(Box::new(ActorTestError::InvalidMessageOrder));
                            }
                        }
                    }
                }

                Behavior::keep()
            })
            .on_tell::<TestActorMessage>(|_msg, state, ctx| -> BehaviorAction<TestActor<S>> {
                // this handlers job is to run the given test tasks
                if let Some(task) = state.tasks.pop_front() {
                    println!("Current task: {:?}", &task);
                    match task {
                        TestTask::Tell(msg, _id) => {
                            state.addr.send(msg);
                            ctx.get_addr().tell(TestActorMessage::RunNext);
                        },
                        TestTask::Ask(mut msg, response, _id) => {
                            // fill in reply_to such that ask queries are responded to this actor
                            msg.sender = Some(ctx.get_addr());
                            state.test_state = TestActorState::PendingResponse(response);

                            // send tell message to actor
                            state.addr.send(msg);
                        },
                        TestTask::Check(check_fn, _id) => {
                            state.addr.ask(StateCheckMessage::<S>::Check(check_fn), ctx.get_addr());
                            state.test_state = PendingResponse(ResponseDyn::Check);
                        }
                        TestTask::Expect(response, _id) => {
                            state.test_state = TestActorState::PendingResponse(response);
                        }
                        TestTask::Exit => {
                            ctx.kill()
                        }
                    }
                }

                // if no message to this handler is rescheduled above the test is done
                Behavior::keep()
            })
    }

}

/// This builder is used to build a [TestActor].
pub struct ActorTestBuilder<S: Send + 'static> {
    behavior_builder: BehaviorBuilder<TestActor<S>>,
    addr: Addr,
    tasks: VecDeque<TestTask<S>>,
    test_state: TestActorState,
    task_id_gen: u32
}

/// This enum represents the possible message types an [Actor] can send.
pub enum MessageType<M> {
    Tell(M),
    Ask(M)
}

impl<S: Send + 'static> ActorTestBuilder<S> {

    /// Creates a new [ActorTestBuilder] with some default settings which are needed to run tests.
    pub fn new(addr: Addr) -> Self {
        let blanket_behavior_builder = TestActor::get_blanket_behavior();

        ActorTestBuilder {
            behavior_builder: blanket_behavior_builder,
            addr: addr,
            tasks: VecDeque::new(),
            test_state: TestActorState::Ready,
            task_id_gen: 0
        }
    }

    /// All tasks are enumerated with a locally (test scope) unique id.
    fn next_task_id(&mut self) -> u32 {
        self.task_id_gen += 1;
        self.task_id_gen -1
    }

    /// Adds the given check function to the test list. check_fn can immutably access the whole
    /// internal state of the actor to be tested.
    pub fn check(mut self, check_fn: fn(&S) -> bool) -> Self {
        let next_id = self.next_task_id();
        self.tasks.push_back(TestTask::Check(check_fn, next_id));
        self
    }

    /// Sends the given message to the actor to be tested.
    pub fn tell<M: Any + Send>(mut self, msg: M) -> Self {
        let msg = Message::without_sender(msg);
        let next_id = self.next_task_id();
        self.tasks.push_back(TestTask::Tell(msg, next_id));
        self
    }

    /// Sends the given message to the actor to be tested and automatically inserts the [TestActor]'s
    /// address into the reply_to field. Further an expected [Response] has to be defined which is also
    /// checked.
    pub fn ask<M: Any + Send, R: Any + Send>(mut self, msg: M, expected_response: Response<R>) -> Self {
        // without_sender is used here since the addr of this TestActor is not known yet and will
        // be filled in later
        let msg = Message::without_sender(msg);
        let expected_response: ResponseDyn = expected_response.into();

        let next_id = self.next_task_id();

        match &expected_response {
            ResponseDyn::Ask(_, _) => {
                self.tasks.push_back(TestTask::Ask(msg, expected_response, next_id));
                self.set_default_ask_response_behavior::<R>()
            }
            ResponseDyn::Tell(_, _) => {
                self.tasks.push_back(TestTask::Ask(msg, expected_response, next_id));
                self.set_default_tell_response_behavior::<R>()
            }
            Check => {
                self.tasks.push_back(TestTask::Ask(msg, expected_response, next_id));
                self
            }
        }
    }

    /// Adds the default tell message handler for a given type M. This is needed such that the TestActor
    /// can receive responses of not yet defined messages.
    fn set_default_tell_response_behavior<M: Any + Send>(mut self) -> Self {
        if !self.behavior_builder.has_tell_handler(TypeId::of::<M>()) {
            // no handler for this type exists yet!

            self.behavior_builder = self.behavior_builder
                .on_tell::<M>(|msg, state, ctx| -> BehaviorAction<TestActor<S>> {

                    match &state.test_state {
                        Ready => {
                            // panic!("did not expect a tell message");
                            return Err(Box::new(ActorTestError::InvalidMessageOrder));
                        }
                        PendingResponse(resp) => {
                            match resp {
                                ResponseDyn::Ask(_, _) => {
                                    // panic!("did not expect an ask message");
                                    return Err(Box::new(ActorTestError::InvalidMessageOrder));
                                }
                                ResponseDyn::Tell(expected_type_id, criteria) => {
                                    if TypeId::of::<M>() == *expected_type_id {
                                        if criteria(Message::without_sender(msg)) == false {
                                            return Err(Box::new(ActorTestError::CriteriaNotMet));
                                            // panic!("tell message did not pass criteria check!");
                                        }
                                    }
                                }
                                Check => {
                                    // panic!("did not expect a check message")
                                    return Err(Box::new(ActorTestError::InvalidMessageOrder));
                                }
                            }
                        }
                    }

                    state.test_state = TestActorState::Ready;
                    // continue working on tasks
                    ctx.get_addr().tell(TestActorMessage::RunNext);
                    Behavior::keep()
                });
        }
        self
    }

    /// Adds the default ask message handler for a given type M. This is needed such that the TestActor
    /// can receive responses of not yet defined messages.
    fn set_default_ask_response_behavior<M: Any + Send>(mut self) -> Self {
        if !self.behavior_builder.has_ask_handler(TypeId::of::<M>()) {
            // no handler for this type exists yet!

            self.behavior_builder = self.behavior_builder
                .on_ask::<M>(|msg, state, _addr, ctx| -> BehaviorAction<TestActor<S>> {

                    match &state.test_state {
                        Ready => {
                            // panic!("did not expect an ask message");
                            return Err(Box::new(ActorTestError::InvalidMessageOrder));
                        }
                        PendingResponse(resp) => {
                            match resp {
                                ResponseDyn::Ask(expected_type_id, criteria) => {
                                    if TypeId::of::<M>() == *expected_type_id {
                                        if criteria(Message::without_sender(msg)) == false {
                                            // panic!("ask message did not pass criteria check!");
                                            return Err(Box::new(ActorTestError::CriteriaNotMet));
                                        }
                                    }
                                }
                                ResponseDyn::Tell(_, _) => {
                                    // panic!("did not expect a tell message");
                                    return Err(Box::new(ActorTestError::InvalidMessageOrder));
                                }
                                Check => {
                                    // panic!("did not expect a check message")
                                    return Err(Box::new(ActorTestError::InvalidMessageOrder));
                                }
                            }
                        }
                    }
                    // continue working on tasks
                    ctx.get_addr().tell(TestActorMessage::RunNext);
                    Behavior::keep()
                });
        }
        self
    }

    /// This function defines that a tell-message is to be received next with the given condition.
    /// If no specific condition is required a simple |msg| true can be passed in.
    pub fn expect_tell<M: Any + Send>(mut self, criteria: fn(M) -> bool) -> Self {
        let task = TestTask::<S>::Expect(ResponseDyn::tell(criteria), self.next_task_id());
        self.tasks.push_back(task);
        self.set_default_tell_response_behavior::<M>()
    }

    /// This function defines that an ask-message is to be received next with the given condition.
    /// If no specific condition is required a simple |msg| true can be passed in.
    pub fn expect_ask<M: Any + Send>(mut self, criteria: fn(M) -> bool) -> Self {
        let task = TestTask::<S>::Expect(ResponseDyn::tell(criteria), self.next_task_id());
        self.tasks.push_back(task);
        self.set_default_tell_response_behavior::<M>()
    }

    /// Consumes the builder and generates an Actor which represents the defined testing behavior.
    pub fn build(mut self) -> Actor<TestActor<S>> {
        // add exit task at end of test tasks
        self.tasks.push_back(TestTask::Exit);

        let state = TestActor {
            addr: self.addr,
            tasks: self.tasks,
            test_state: TestActorState::Ready
        };

        Actor::new(state, self.behavior_builder.build(), MailboxType::Unbounded)

    }

}


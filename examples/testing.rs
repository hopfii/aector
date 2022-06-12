use std::any::Any;
use std::collections::VecDeque;
use std::time::Duration;
use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::behavior::{ActorManageMessage, Behavior, BehaviorBuilder, BehaviorAction, StateCheckMessage};
use aector::{Addr, Message};
use aector::testing::{MessageType, Response, TestActor, ActorTestBuilder};
use crate::Direction::{DOWN, UP};

fn main() {}

#[derive(PartialEq)]
enum Direction {
    UP, DOWN
}

struct SimpleState {
    state: Direction
}

enum SimpleMessage {
    SetUp, SetDown
}

#[cfg(test)]
mod tests {
    use aector::actor::MailboxType;
    use super::*;

    // tests have to be tagged with tokio::test such that those are run with an async runtime
    #[tokio::test]
    async fn main2() {

        // define simple example actor for showing testing functions
        let simple_state = SimpleState {
            state: UP
        };

        let simple_behavior = BehaviorBuilder::new()
            .on_start(|state: &mut SimpleState, ctx| {
                state.state = DOWN;
            })
            .on_ask::<SimpleMessage>(|msg, state, reply_to, ctx| -> BehaviorAction<SimpleState> {
                match msg {
                    SimpleMessage::SetUp => {
                        state.state = UP;
                    }
                    SimpleMessage::SetDown => {
                        state.state = DOWN;
                    }
                }

                reply_to.tell("OK".to_string());

                Behavior::keep()
            })
            .on_ask::<String>(|msg, state, reply_to, ctx| -> BehaviorAction<SimpleState> {

                if msg == "ECHO" {
                    reply_to.tell("ECHO ECHO".to_string());
                } else {
                    reply_to.tell(msg);
                }

                Behavior::keep()
            })
            .enable_state_checks()
            .build();


        let actor = Actor::new(simple_state, simple_behavior, MailboxType::Unbounded);
        let addr = actor.get_addr();

        let sys = ActorSystem::new();
        sys.spawn(actor, "actor to be tested".to_string());

        // define test actor
        let test_actor = ActorTestBuilder::new(addr)
            .check(|state: &SimpleState| state.state == DOWN)
            .ask(SimpleMessage::SetUp, Response::Tell(|msg: String| msg == "OK".to_string()))
            .check(|state: &SimpleState| state.state == UP)
            .ask("ECHO".to_string(), Response::Tell(|msg: String| msg == "ECHO ECHO".to_string()))
            .ask("RANDOM".to_string(), Response::Tell(|msg: String| msg == "RANDOM".to_string()))
            .tell(ActorManageMessage::Kill)
            .build();

        // get test result: true = test passed, false = test failed
        let test_res = sys.spawn_test(test_actor).await;
        assert_eq!(test_res, true);

        sys.start().await;

    }

}

#[tokio::test]
async fn simple_actor_test() {
    // define a simple behavior for an i32 state
    let behavior = BehaviorBuilder::new()
        .on_tell::<i32>(|msg, state, ctx| -> BehaviorAction<i32> {
            *state += msg;
            Behavior::keep()
        })
        .enable_state_checks()
        .build();
    let actor = Actor::new(0, behavior, MailboxType::Unbounded);
    let addr = actor.get_addr();

    // create an empty actorsystem
    let sys = ActorSystem::new();
    sys.spawn(actor, "actor to be tested".to_string());

    // define test
    let test_actor = ActorTestBuilder::new(addr)
        .check(|state: &i32| *state == 0)
        .tell(10)
        .check(|state| *state == 10)
        .tell(ActorManageMessage::Kill)
        .build();

    let test_res = sys.spawn_test(test_actor).await;
    assert_eq!(test_res, true);

    // start actorsystem to run actors
    sys.start().await;
}


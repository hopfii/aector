//! The functions and structs contained within this module can be used to test the behavior of an
//! actor in a sequential way.
//!
//! In order to be able to test an actor, every behavior has to support specific testing messages.
//! These can be enabled with the .enable_state_checks() method on the BehaviorBuilder.
//!
//! Example:
//! ```
//! use aector::actor::{Actor, MailboxType};
//! use aector::actor_system::ActorSystem;
//! use aector::behavior::{ActorManageMessage, Behavior, BehaviorBuilder, BehaviorAction};
//! use aector::testing::ActorTestBuilder;
//! #[tokio::test]
//! async fn simple_actor_test() {
//!     // define a simple behavior for an i32 state
//!     let behavior = BehaviorBuilder::new()
//!         .on_tell::<i32>(|msg, state, ctx| -> BehaviorAction<i32> {
//!             *state += msg;
//!             Behavior::keep()
//!         })
//!         .enable_state_checks()
//!         .build();
//!
//!     let actor = Actor::new(0, behavior, MailboxType::Unbounded);
//!     let addr = actor.get_addr();
//!
//!     // create an empty actorsystem
//!     let sys = ActorSystem::new();
//!     sys.spawn(actor, "actor to be tested".to_string());
//!
//!     // define test
//!     let test_actor = ActorTestBuilder::new(addr)
//!         .check(|state: &i32| *state == 0)
//!         .tell(10)
//!         .check(|state| *state == 10)
//!         .tell(ActorManageMessage::Kill)
//!         .build();
//!
//!     let test_res = sys.spawn_test(test_actor).await;
//!     assert_eq!(test_res, true);
//!
//!     // start actor system to run actors
//!     sys.start().await;
//!
//! }
//! ```
//!

mod actor_test;
pub use actor_test::{TestActor, ActorTestBuilder, Response, MessageType};


//! This module contains all data structures for creating and using actors and their
//! respective functionality.
//! All actors consist of an inital state and behavior. Example:
//! ```
//! use aector::actor::{Actor, MailboxType};
//! use aector::behavior::{Behavior, BehaviorBuilder, BehaviorAction};
//!
//! struct PingPong {}
//! // enum used as message
//! enum Ball { Ping, Pong }
//!
//! // define behavior for actor
//! let mut behavior = BehaviorBuilder::new()
//!     .on_ask::<Ball>(|msg, state, sender, ctx| -> BehaviorAction<PingPong> {
//!         match msg {
//!             Ball::Ping => {
//!                 sender.ask(Ball::Pong, ctx.get_addr());
//!             },
//!             Ball::Pong => {
//!                 sender.ask(Ball::Ping, ctx.get_addr());
//!             }
//!         }
//!         Behavior::keep()
//!     })
//!     .build();
//! let mut actor = Actor::new(PingPong {}, behavior, MailboxType::Unbounded);
//! let addr = actor1.get_addr();
//! ```

mod actor;
mod backup;
mod actor_context;
mod mailbox;

pub use actor::{Actor, ExitReason, MailboxType};
pub use backup::Backup;
pub use actor_context::{ActorContext};


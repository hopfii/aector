//! This module contains structs and traits which are relevant for using and creating supervision
//! strategies.
//! All actors can be spawned with a given supervision strategy:
//! ```
//! use aector::actor::{Actor, MailboxType};
//! use aector::behavior::BehaviorBuilder;
//! use aector::supervision::strategies::SimpleRestartStrategy;
//!
//! let actor = Actor::new((), BehaviorBuilder::new().build(), MailboxType::Unbounded);
//! actor_sys.spawn_with_supervision(actor, SimpleRestartStrategy::new(), "supervised actor".to_string());
//! ```


mod simple_restart_strategy;
mod supervision;

pub use supervision::{SupervisionStrategy, SuperVisionAction};
pub mod strategies {
    pub use super::simple_restart_strategy::SimpleRestartStrategy;
}

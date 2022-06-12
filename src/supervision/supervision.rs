use std::time::Duration;

use crate::actor::{Actor, ExitReason};
use crate::actor::Backup;

/// Represents decision of SuperVisionStrategy
#[derive(Debug)]
pub enum SuperVisionAction {
    Exit,
    Restart,
    RestartDelayed(Duration)
}

/// All supervision strategies have to implement this trait in order to be used as a supervision
/// strategy in this framework.
pub trait SupervisionStrategy<S: Send + Clone> {
    /// This function is applied when an actor exits its regular run loop. The return value describes
    /// the action to be taken by the actor system for this specific actor.
    fn apply(&mut self, exit_reason: ExitReason, backup: &Backup<S>, actor: &mut Actor<S>) -> SuperVisionAction;
}


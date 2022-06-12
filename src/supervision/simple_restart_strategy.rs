use crate::actor::{Actor, ExitReason};
use crate::actor::Backup;
use crate::supervision::supervision::{SuperVisionAction, SupervisionStrategy};
use crate::supervision::supervision::SuperVisionAction::{Exit, Restart};

/// Implements a simple restart strategy where the supervised actor is instantly restarted unless
/// the actor requested the stop itself, in which case the actor is stopped and removed from
/// the actor system.
pub struct SimpleRestartStrategy {}

impl SimpleRestartStrategy {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl<S: Send + Clone> SupervisionStrategy<S> for SimpleRestartStrategy {
    fn apply(&mut self, exit_reason: ExitReason, backup: &Backup<S>, actor: &mut Actor<S>) -> SuperVisionAction {
        match exit_reason {
            ExitReason::Kill => {
                println!("ActorSys: actor died on purpose");
                return Exit;
            }
            ExitReason::Restart => {
                println!("ActorSys: actor requested a restart. Restarting actor with initial state and behavior");
                actor.apply_backup(&backup);
                return Restart;
            },
            ExitReason::Error => {
                println!("ActorSys: actor ran into error or triggered restart. Restarting actor with initial state and behavior");
                actor.apply_backup(&backup);
                return Restart;
            }
        }
    }
}

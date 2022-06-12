use crate::behavior::Behavior;

/// Represents a snapshot of a generic, cloneable state S and a [Behavior] of an [Actor](crate::actor::Actor).
/// This is used for restoring the initial state and [Behavior] of an [Actor](crate::actor::Actor) if a
/// SupervisionStrategy decides to restart the [Actor](crate::actor::Actor).
pub struct Backup<S: Send + Clone + 'static> {
    state: S,
    behavior: Behavior<S>
}

impl<S: Send + Clone + 'static> Backup<S> {
    pub(crate) fn new(state: S, behavior: Behavior<S>) -> Self {
        Self {
            state,
            behavior
        }
    }

    /// Returns a clone of the stored state.
    pub(crate) fn get_state(&self) -> S {
        self.state.clone()
    }

    /// Returns a clone of the stored behavior.
    pub(crate) fn get_behavior(&self) -> Behavior<S> {
        self.behavior.clone()
    }
}

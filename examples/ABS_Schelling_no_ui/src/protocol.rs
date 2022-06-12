use crate::person::{PopulationType};
use crate::grid::{Pos, GridT};

pub struct GetInitialPos {
    pub race: PopulationType
}
pub struct RequestNewPos {
    pub race: PopulationType,
    pub old_pos: Pos
}
pub struct NewPos {
    pub new_pos: Pos
}

pub struct InitPos {
    pub pos: Pos
}

pub struct GetGrid {}
pub struct GridSnapshot {
    pub grid: Box<GridT>
}

#[derive(Clone)]
pub struct ExecuteStep {}
pub struct StepDone {}

pub struct ExecuteSimStep {}

pub struct InitDone {}
pub struct PrintGrid {}

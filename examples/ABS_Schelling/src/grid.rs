use std::cmp;
use std::sync::LockResult;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::Addr;
use aector::behavior::{Behavior, BehaviorAction, BehaviorBuilder};
use rand::Rng;
use rand::rngs::ThreadRng;
use rand::SeedableRng;
use rand_isaac::Isaac64Rng;

use crate::N;
use crate::person::PopulationType;
use crate::protocol::*;

pub type Pos = (usize, usize);
pub type GridT = [[Option<PopulationType>; N]; N];

pub struct Grid {
    grid: Box<GridT>,
    free_positions: Vec<Pos>,
    rng: Isaac64Rng
}

impl Grid {
    pub fn new() -> Actor<Self> {

        let mut free_positions = Vec::new();
        for x in 0..N {
            for y in 0..N {
                free_positions.push((x, y));
            }
        }

        let g = Box::new([[None; N];N]);

        let grid = Grid {
            grid: g,
            free_positions,
            rng: rand_isaac::Isaac64Rng::from_entropy()
        };


        let mut grid_behavior = BehaviorBuilder::new()
            .on_ask::<GetInitialPos>(|msg, state, reply_to, ctx| -> BehaviorAction<Grid> {
                let new_pos = state.get_random_free_pos();
                let (x, y) = new_pos;
                state.grid[y][x] = Some(msg.race);

                reply_to.tell(InitPos{pos: new_pos});
                Behavior::keep()
            })
            .on_ask::<RequestNewPos>(|msg, state, reply_to, ctx| -> BehaviorAction<Grid> {
                let new_pos = state.get_random_free_pos();
                let (x, y) = new_pos;
                state.grid[y][x] = Some(msg.race);

                let (x_old, y_old) = msg.old_pos;
                state.grid[y_old][x_old] = None;
                state.free_positions.push(msg.old_pos);
                reply_to.tell(NewPos{new_pos});
                Behavior::keep()
            })
            .on_ask::<GetGrid>(|msg, state, reply_to, ctx| -> BehaviorAction<Grid> {
                reply_to.tell(GridSnapshot{grid: state.grid.clone()});
                Behavior::keep()
            })
            .on_tell::<PrintGrid>(|msg, state, ctx| -> BehaviorAction<Grid> {
                let ui = ctx.query("ui").expect("could not find ui actor!");
                ui.tell(GridSnapshot { grid: state.grid.clone()});

                Behavior::keep()
            })
            .build();

        Actor::new(grid, grid_behavior, MailboxType::Unbounded)
    }

    fn get_random_free_pos(&mut self) -> Pos {
        let idx = self.rng.gen_range(0..self.free_positions.len());
        self.free_positions.swap_remove(idx)
    }
}

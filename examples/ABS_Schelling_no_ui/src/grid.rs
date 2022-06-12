use crate::protocol::*;
use crate::person::PopulationType;
use std::cmp;
use std::fs::File;
use std::io::Write;
use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::Addr;
use aector::behavior::{Behavior, BehaviorAction, BehaviorBuilder};
use rand::rngs::ThreadRng;
use rand_isaac::Isaac64Rng;
use rand::SeedableRng;
use rand::Rng;
use crate::N;
use crate::protocol::*;

// type used to internally represent the NxN grid as 2D array
pub type GridT = [[Option<PopulationType>; N]; N];
pub type Pos = (usize, usize);
pub struct Grid {
    grid: Box<GridT>,
    free_positions: Vec<Pos>,
    rng: Isaac64Rng
}

impl Grid {
    /// Creates new Grid-Actor with an empty grid.
    pub fn new() -> Actor<Self> {
        let mut free_positions = Vec::with_capacity(N*N);
        for x in 0..N {
            for y in 0..N {
                free_positions.push((x, y));
            }
        }

        let grid = Grid {
            grid: Box::new([[None; N];N]),
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
                // store grid output to a file
                let mut file = File::create("grid_output.txt").expect("Failed to open grid output file!");

                for y in 0..N {
                    let mut line = String::with_capacity(N+1);
                    for x in 0..N {
                        match state.grid[y][x] {
                            None => {
                                line.insert(x, ' ');
                                // print!(" ");
                            },
                            Some(race) => {
                                match race {
                                    PopulationType::A => {
                                        // print!("A");
                                        line.insert(x, 'A');
                                    },
                                    PopulationType::B => {
                                        // print!("B");
                                        line.insert(x, 'B');
                                    }
                                }
                            }
                        }
                    }
                    line.insert(N, '\n');
                    file.write_all(&line.into_bytes());
                }

                ctx.stop();

                Behavior::keep()
            })
            .build();

        Actor::new(grid, grid_behavior, MailboxType::Unbounded)
    }

    /// Returns a random free position in the grid and removes it from the free_positions list.
    fn get_random_free_pos(&mut self) -> Pos {
        let idx = self.rng.gen_range(0..self.free_positions.len());
        self.free_positions.swap_remove(idx)
    }
}

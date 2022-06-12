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
use crate::grid::*;
use crate::protocol::*;

#[derive(Clone, Copy)]
pub enum PopulationType {
    A,
    B
}
pub struct Person {
    id: u32,
    population_type: PopulationType,
    pos: Option<Pos>,
    grid_actor: Addr,
    sim_actor: Addr,
    min_happiness: f32,
    neighbourhood_size: i32
}

impl Person {
    /// Decides whether the Person wants to move based on its neighbourhood
    fn should_move(&self, grid: GridSnapshot) -> bool {
        let grid = grid.grid;
        let (x, y) = self.pos.expect("Cant calculate happiness of a person which has no position!");

        let x_min = cmp::max(0i32, (x as i32) - self.neighbourhood_size) as usize;
        let x_max = cmp::min((N as i32) -1, (x as i32) + self.neighbourhood_size) as usize;
        let y_min = cmp::max(0i32, (y as i32) - self.neighbourhood_size) as usize;
        let y_max = cmp::min((N as i32) -1, (y as i32) + self.neighbourhood_size) as usize;

        let mut empty = 0;
        let mut race_a_count = 0;
        let mut race_b_count = 0;

        for y in y_min..=y_max {
            for x in x_min..=x_max {
                match grid[y][x] {
                    Some(race) => {
                        match race {
                            PopulationType::A => {
                                race_a_count += 1;
                            },
                            PopulationType::B => {
                                race_b_count += 1;
                            }
                        }
                    },
                    None => {
                        empty += 1;
                    }
                }
            }
        }

        let ratio;
        // note: ignoring empty fields here!
        let total = (race_a_count + race_b_count) as f32;
        match self.population_type {
            PopulationType::A => {
                ratio = (race_a_count as f32) / total;
            },
            PopulationType::B => {
                ratio = (race_b_count as f32) / total;
            }
        }

        return ratio >= self.min_happiness;
    }

    pub fn new(id: u32, race: PopulationType, grid_actor: Addr, sim_actor: Addr, min_happiness: f32, neighbourhood_size: i32) -> Actor<Self> {
        let p = Person {
            id,
            population_type: race,
            pos: None,
            grid_actor,
            sim_actor,
            min_happiness,
            neighbourhood_size
        };
        let mut person_behavior = BehaviorBuilder::new()
            .on_start(|state: &mut Person, ctx| {
                // get initial position
                state.grid_actor.ask(GetInitialPos {race: state.population_type }, ctx.get_addr());
            })
            .on_tell::<InitPos>(|msg, state, ctx| -> BehaviorAction<Person> {
                // received a new requested position
                state.pos = Some(msg.pos);
                state.sim_actor.tell(InitDone{});
                Behavior::keep()
            })
            .on_tell::<NewPos>(|msg, state, ctx| -> BehaviorAction<Person> {
                // received a new requested position
                state.pos = Some(msg.new_pos);
                state.sim_actor.tell(StepDone{});
                Behavior::keep()
            })
            .on_tell::<ExecuteStep>(|msg, state, ctx| -> BehaviorAction<Person> {
                // trigger for executing a sim step
                state.grid_actor.ask(GetGrid{}, ctx.get_addr());
                Behavior::keep()
            })
            .on_tell::<GridSnapshot>(|msg, state, ctx| -> BehaviorAction<Person> {
                // received requested grid snapshot
                if state.should_move(msg) {
                    // give back current pos to grid
                    let old_pos = state.pos.take().expect("Person has no position yet!");
                    // request a new pos from grid
                    state.grid_actor.ask(RequestNewPos{old_pos, race: state.population_type }, ctx.get_addr());
                } else {
                    // person is happy, no new pos needed
                    state.sim_actor.tell(StepDone{});
                }
                Behavior::keep()
            })
            .build();

        Actor::new(p, person_behavior, MailboxType::Unbounded)
    }
}

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
use crate::protocol::*;

pub struct Sim {
    steps_done: u32,
    number_of_persons: u32,
    received_stepdone_messages: u32,
    max_num_of_steps: u32,
    init_done: bool,
    inits_received: u32
}

impl Sim {
    pub fn new(num_of_persons: u32, max_num_of_steps: u32) -> Actor<Self> {
        let sim = Sim {
            steps_done: 0,
            number_of_persons: num_of_persons,
            received_stepdone_messages: 0,
            max_num_of_steps,
            init_done: false,
            inits_received: 0
        };
        let mut sim_behavior = BehaviorBuilder::new()
            .on_tell::<InitDone>(|msg, state, ctx| -> BehaviorAction<Sim> {
                state.inits_received += 1;

                if state.inits_received == state.number_of_persons {
                    ctx.get_addr().tell(ExecuteSimStep{});
                }
                Behavior::keep()
            })
            .on_tell::<StepDone>(|msg, state, ctx| -> BehaviorAction<Sim> {
                state.received_stepdone_messages += 1;

                if state.received_stepdone_messages >= state.number_of_persons {
                    state.steps_done += 1;
                    // dbg!("Sim step {} done", state.steps_done);
                    if state.steps_done >= state.max_num_of_steps {
                        // dbg!("Sim done!");

                        // commented out for benchmarking
                        // let grid = ctx.query("grid").expect("could not find grid actor!");
                        // grid.tell(PrintGrid{});

                        ctx.stop();
                    } else {
                        // reschedule next sim step
                        ctx.get_addr().tell(ExecuteSimStep{});
                    }
                }

                Behavior::keep()
            })
            .on_tell::<ExecuteSimStep>(|msg, state, ctx| -> BehaviorAction<Sim> {
                state.received_stepdone_messages = 0;
                ctx.broadcast_tell(ExecuteStep{});
                Behavior::keep()
            })
            .build();

        Actor::new(sim, sim_behavior, MailboxType::Unbounded)
    }
}

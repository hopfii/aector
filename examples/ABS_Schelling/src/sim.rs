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


use crate::protocol::*;

pub struct Sim {
    steps_done: u32,
    number_of_persons: u32,
    received_stepdone_messages: u32,
    max_num_of_steps: u32,
    init_done: bool,
    inits_received: u32,
    delay: Duration
}

impl Sim {
    pub fn new(num_of_persons: u32, max_num_of_steps: u32, delay: Duration) -> Actor<Self> {
        let sim = Sim {
            steps_done: 0,
            number_of_persons: num_of_persons,
            received_stepdone_messages: 0,
            max_num_of_steps,
            init_done: false,
            inits_received: 0,
            delay: delay
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
                    println!("Sim: received all StepDone messages for iteration {}!", state.steps_done);

                    // tell grid actor to send a snapshot of grid to UI actor
                    let grid = ctx.query("grid").expect("could not find grid actor!");
                    grid.tell(PrintGrid{});

                    if state.steps_done >= state.max_num_of_steps {
                        println!("Max sim steps of {} reached!", state.max_num_of_steps);
                    } else {
                        // reschedule next sim step
                        ctx.get_addr().tell_delayed(ExecuteSimStep{}, state.delay);
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

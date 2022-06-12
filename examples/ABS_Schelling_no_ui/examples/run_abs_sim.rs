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

/*
#Person.on_start:
Person  ->  Grid: GetInitialPos
Person  <-  Grid: InitPos(pos)
Person  ->  Sim: InitDone

if Sim received all inits: schedule ExecuteSimStep to self

#ExecuteSimStep:
Sim     ->  Sim:        ExecuteSimStep
Sim     ->  Broadcast:  ExecuteStep
Person  ->  Grid:       GetGrid
Person  <-  Grid:       GridSnapshot

# if Person.happy > Person.min_happy:
Person  ->  Sim:        StepDone

# else
Person  ->  Grid:       RequestNewPos(old_pos)
Person  <-  Grid:       NewPos(new_pos)
Person  ->  Sim:        StepDone

# Sim during step:
Person  ->  Sim:        StepDone
if #stepdones == #persons: sim step done
else: #stepdones += 1
 */

use ABS_Schelling_no_ui::protocol::*;
use ABS_Schelling_no_ui::sim::*;
use ABS_Schelling_no_ui::grid::*;
use ABS_Schelling_no_ui::person::*;
use tokio::runtime::Builder;

async fn run_bench_sim() {

    let actor_sys = ActorSystem::new();
    let num_of_persons = 128000;
    let neighbourhood_size = 5;
    let min_happiness = 0.6;

    let sim = Sim::new(num_of_persons, 100);
    let grid = Grid::new();

    let sim_addr = sim.get_addr();
    let grid_addr = grid.get_addr();

    actor_sys.spawn(sim, "sim".to_string());
    actor_sys.spawn(grid, "grid".to_string());

    for i in 0..num_of_persons {
        let race;
        // population is of equal size
        if i % 2 == 0 {
            race = PopulationType::A;
        } else {
            race = PopulationType::B;
        }

        let person = Person::new(i, race, grid_addr.clone(), sim_addr.clone(), min_happiness, neighbourhood_size);
        actor_sys.spawn(person, i.to_string());
    }

    actor_sys.start().await;
}

fn main() {
    let rt = Builder::new_multi_thread()
        .worker_threads(10)
        .thread_stack_size(2 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        run_bench_sim().await;
    })
}



use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use aector::actor_system::ActorSystem;
use tokio::runtime::Builder;

use ABS_Schelling::grid::Grid;
use ABS_Schelling::N;
use ABS_Schelling::person::{Person, PopulationType};
use ABS_Schelling::sim::Sim;
use ABS_Schelling::ui::{UI, UIGridData};

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
if #stepdones == #persons:
    sim step done
    Sim ->  Grid:       PrintGrid
    Grid->  UI:         GridSnapshot(grid)
else:
    #stepdones += 1
 */


// Grid size = NxN
const NEIGHBOURHOOD_SIZE: i32 = 1;
const MIN_HAPPINESS: f32 = 0.4;

async fn run_sim() {
    
    let actor_sys = ActorSystem::new();
    let num_of_persons = 2000;

    let sim = Sim::new(num_of_persons, 100, Duration::from_millis(0));
    let grid = Grid::new();

    let ui_grid_store = Arc::new(Mutex::new(None));
    let ui = UIGridData::new(ui_grid_store.clone());

    // run sdl in own thread
    let jh = thread::spawn(move || {
        let mut a = UI::new(ui_grid_store, 1000, N as u32);
        a.run();
    });


    let sim_addr = sim.get_addr();
    let grid_addr = grid.get_addr();

    actor_sys.spawn(sim, "sim".to_string());
    actor_sys.spawn(grid, "grid".to_string());
    actor_sys.spawn(ui, "ui".to_string());

    for i in 0..num_of_persons {
        let race;
        // population is of equal size
        if i % 2 == 0 {
            race = PopulationType::A;
        } else {
            race = PopulationType::B;
        }

        let person = Person::new(i, race, grid_addr.clone(), sim_addr.clone(), MIN_HAPPINESS, NEIGHBOURHOOD_SIZE);
        actor_sys.spawn(person, i.to_string());
    }

    actor_sys.start().await;
    jh.join();
}

pub fn main() {
    let rt = Builder::new_multi_thread()
        .worker_threads(10)
        .thread_stack_size(2 * 1024 * 1024)
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        run_sim().await;
    })
}


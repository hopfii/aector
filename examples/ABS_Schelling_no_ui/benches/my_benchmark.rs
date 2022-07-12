use std::time::Duration;
use ABS_Schelling_no_ui::protocol::*;
use ABS_Schelling_no_ui::sim::*;
use ABS_Schelling_no_ui::grid::*;
use ABS_Schelling_no_ui::person::*;
use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::Addr;
use aector::behavior::{Behavior, BehaviorAction, BehaviorBuilder};
use tokio::runtime::Builder;


async fn run_bench_sim() {

    let actor_sys = ActorSystem::new();
    let num_of_persons = 2000;
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
    println!("done");
}

fn run_sim() {
    let rt = Builder::new_multi_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        run_bench_sim().await;
    })
}

use criterion::{black_box, criterion_group, criterion_main, Criterion, Benchmark};
use criterion::async_executor::AsyncExecutor;

pub fn criterion_benchmark(c: &mut Criterion) {

    //             .measurement_time(Duration::from_secs(100))
    c.bench(
        "bench",
        Benchmark::new("schelling 100", |b| {
            b.iter(|| run_sim());
        })
            .sample_size(20)
    );

}



criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);


use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use aector::actor::{Actor, MailboxType};
use aector::actor_system::ActorSystem;
use aector::behavior::{Behavior, BehaviorBuilder, BehaviorAction};
use aector::supervision::strategies::SimpleRestartStrategy;


#[tokio::main]
async fn main() {
    type SharedState = Arc<Mutex<String>>;
    let mut behavior = BehaviorBuilder::new()
        .on_tell::<()>(|msg, state, ctx| -> BehaviorAction<SharedState> {
            // state has been externally cleared
            println!("State: {}", state.lock().unwrap());
            Behavior::keep()
        })
        .build();

    let shared_state = Arc::new(Mutex::new(String::from("hello world!")));
    let actor = Actor::new(shared_state.clone(), behavior, MailboxType::Unbounded);
    let tx = actor.get_addr();

    let actor_sys = ActorSystem::new();
    actor_sys.spawn(actor,  "actor".to_owned());

    // clear state
    shared_state.lock().unwrap().clear();
    tx.tell(());
    actor_sys.start().await;
}



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

    type SharedMsg = Arc<Mutex<String>>;
    let mut behavior = BehaviorBuilder::new()
        .on_tell::<SharedMsg>(|msg, state, ctx| -> BehaviorAction<()> {
            let mut str = msg.lock().unwrap();
            str.clear();
            ctx.kill();
            Behavior::keep()
        })
        .build();

    let shared_str = Arc::new(Mutex::new(String::from("hello world!")));
    let actor = Actor::new((), behavior, MailboxType::Unbounded);
    let tx = actor.get_addr();

    let actor_sys = ActorSystem::new();
    actor_sys.spawn(actor,  "actor".to_owned());
    tx.tell(shared_str.clone());
    actor_sys.start().await;
    println!("{}", shared_str.lock().unwrap());
}




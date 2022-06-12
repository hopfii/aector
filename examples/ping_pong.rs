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
    // setup logging
    let app_name = "demo".to_owned();
    LogTracer::init().expect("Unable to setup log tracer!");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);
    let subscriber = Registry::default()
        .with(EnvFilter::new("INFO"))
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // no state is needed for this example, thus the struct is empty
    // has to implement Clone such that the behavior can be cloned
    #[derive(Clone)]
    struct PingPong {}
    // enum used as message
    enum Ball { Ping, Pong }

    // define behavior for actors
    let mut behavior1 = BehaviorBuilder::new()
        .on_ask::<Ball>(|msg, state, sender, ctx| -> BehaviorAction<PingPong> {
            match msg {
                Ball::Ping => {
                    sender.ask(Ball::Pong, ctx.get_addr());
                    println!("Ping");
                },
                Ball::Pong => {
                    sender.ask(Ball::Ping, ctx.get_addr());
                    println!("Pong");
                }
            }
            Behavior::keep()
        })
        .build();

    // second actor has the same behavior
    let behavior2 = behavior1.clone();

    let mut actor1 = Actor::new(PingPong {}, behavior1, MailboxType::Unbounded);
    let tx1 = actor1.get_addr();

    let mut actor2 = Actor::new(PingPong {}, behavior2, MailboxType::Unbounded);
    let tx2 = actor2.get_addr();

    let mut actor_sys = ActorSystem::new();
    actor_sys.spawn(actor1,  "a1".to_owned());
    actor_sys.spawn(actor2,  "a2".to_owned());
    tx1.ask(Ball::Ping, tx2);
    actor_sys.start().await;
}



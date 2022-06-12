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

    // definition of states and messages
    struct Simple {}
    enum Beep {
        Beep
    }

    // state has to implement Clone if used in combination with a supervision strategy
    #[derive(Clone)]
    struct MyState {
        x: String
    }

    // message has to implement clone if used within a broadcast message
    #[derive(Clone)]
    enum Ball {
        Ping,
        Pong
    }

    struct ChangeMsg {}

    let mut behavior = BehaviorBuilder::new()
        .on_start(|state, ctx| {
            println!("on start!");
        })
        .on_ask::<Ball>(|msg, state, sender, ctx| -> BehaviorAction<MyState> {
            match msg {
                Ball::Ping => {
                    println!("PING on_ask");
                    state.x = "PING".to_owned();
                    sender.ask(Ball::Ping, ctx.get_addr());
                }
                Ball::Pong => {
                    println!("PONG on_ask");
                    state.x = "PONG".to_owned()
                }
            }

            println!("New state: {}", &state.x);

            let new_actor_addr = ctx.query("test");
            if let Some(addr) = new_actor_addr {
                println!("Found addr of test-actor!");
                addr.tell("Hello test".to_owned());
            } else {
                println!("Actor test does not exist yet..");
            }

            Behavior::keep()

        })
        .on_tell::<Ball>(|msg, state, ctx| {
            println!("Received ball message on_tell");
            Behavior::keep()
        })
        .on_ask::<ChangeMsg>(|msg, state, sender, ctx| -> BehaviorAction<MyState> {
            println!("received changemsg in on_ask, not responding anymore now because of new behavior!");

            // return empty new behavior
            Behavior::change(BehaviorBuilder::new()
                .build())
        })
        .build();

    let mut state = MyState {
        x: "hello".to_owned()
    };
    let mut my_actor = Actor::new(state, behavior, MailboxType::Unbounded);
    let tx = my_actor.get_addr();


    let behavior2 = BehaviorBuilder::new()
        .on_ask::<Ball>(|msg, state, sender, ctx| -> BehaviorAction<MyState> {
            println!("Answering from a2 with ping");
            sender.ask_delayed(Ball::Ping, ctx.get_addr(), Duration::from_secs(1));

            ctx.run_delayed(Box::new(move || {
                println!("DELAYED STUFF!!");
                sender.tell(Ball::Ping);
            }), Duration::from_secs(4));


            ctx.spawn(Actor::new(Some(3), BehaviorBuilder::new()
                .on_tell::<String>(|msg, state, ctx| -> BehaviorAction<Option<i32>> {
                    println!("TEST: received {}", msg);
                    Behavior::keep()
                })
                .build(), MailboxType::Unbounded), "test".to_owned());

            ctx.kill();
            Behavior::keep()
        })
        .on_kill(|state, ctx| {
            println!("k thx bye from on_kill")
        })
        .build();

    let mut state2 = MyState {
        x: "hello".to_owned()
    };
    let mut my_actor2 = Actor::new(state2, behavior2, MailboxType::Unbounded);

    let tx2 = my_actor2.get_addr();

    let mut actor_sys = ActorSystem::new();

    actor_sys.spawn_with_supervision(my_actor, SimpleRestartStrategy::new(), "a0".to_owned());
    actor_sys.spawn_with_supervision(my_actor2, SimpleRestartStrategy::new(), "a1".to_owned());

    actor_sys.broadcast_tell(Ball::Ping);
    actor_sys.broadcast_ask(Ball::Ping, tx2.clone());

    tx.ask(Ball::Ping, tx2);

    actor_sys.start().await;

}


//! This library provides an implementation of the actor model. The goal of this library is to
//! enable users to create actors, which can dynamically change their behavior during runtime.
//! This is achieved by internally working with trait objects and the Any trait.
//! Further a testing framework is supplied which can be used to generate test actors in an easy
//! and typesafe way.

//! For examples check out the provided examples in the [repository of the library](https://github.com/hopfii/aector).

extern crate core;

pub mod actor_system;
pub mod supervision;
mod address;
pub mod actor;
mod message;
pub mod behavior;

pub mod testing;

pub use address::Addr;
pub use message::Message;



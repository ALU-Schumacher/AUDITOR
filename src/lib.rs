#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod client;
pub mod configuration;
pub mod domain;
pub mod routes;
pub mod startup;
pub mod telemetry;

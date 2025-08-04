// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(feature = "server")]
pub mod archive;
#[cfg(feature = "server")]
pub mod configuration;
pub mod constants;
pub mod domain;
pub mod error;
#[cfg(feature = "server")]
pub mod metrics;
#[macro_use]
mod macros;
pub mod middleware;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "server")]
pub mod startup;
pub mod telemetry;

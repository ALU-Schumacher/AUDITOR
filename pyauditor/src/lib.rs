// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(clippy::borrow_deref_ref)]

use pyo3::prelude::*;

mod blocking_client;
mod builder;
mod client;
mod domain;
mod queued_client;

/// pyauditor is a client for interacting with an Auditor instance via Python.
#[pymodule]
fn pyauditor(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<crate::builder::AuditorClientBuilder>()?;
    m.add_class::<crate::client::AuditorClient>()?;
    m.add_class::<crate::client::Value>()?;
    m.add_class::<crate::client::Operator>()?;
    m.add_class::<crate::client::QueryBuilder>()?;
    m.add_class::<crate::client::MetaQuery>()?;
    m.add_class::<crate::client::MetaOperator>()?;
    m.add_class::<crate::client::ComponentQuery>()?;
    m.add_class::<crate::client::SortBy>()?;
    m.add_class::<crate::blocking_client::AuditorClientBlocking>()?;
    m.add_class::<crate::queued_client::QueuedAuditorClient>()?;
    m.add_class::<crate::domain::Record>()?;
    m.add_class::<crate::domain::Meta>()?;
    m.add_class::<crate::domain::Component>()?;
    m.add_class::<crate::domain::Score>()?;
    Ok(())
}

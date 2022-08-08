// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Score;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct Component {
    pub(crate) inner: auditor::domain::Component,
}

#[pymethods]
impl Component {
    #[new]
    pub fn new(name: String, amount: i64, scores: Vec<Score>) -> Result<Self, anyhow::Error> {
        Ok(Component {
            inner: auditor::domain::Component::new(
                name,
                amount,
                scores.iter().map(|s| s.inner.clone()).collect(),
            )?,
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.as_ref().to_string()
    }

    #[getter]
    fn amount(&self) -> i64 {
        *self.inner.amount.as_ref()
    }

    #[getter]
    fn scores(&self) -> Vec<Score> {
        self.inner.scores.iter().cloned().map(Score::from).collect()
    }
}

impl From<auditor::domain::Component> for Component {
    fn from(component: auditor::domain::Component) -> Component {
        Component { inner: component }
    }
}

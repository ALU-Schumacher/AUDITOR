// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct Score {
    pub(crate) inner: auditor::domain::Score,
}

impl From<auditor::domain::Score> for Score {
    fn from(score: auditor::domain::Score) -> Score {
        Score { inner: score }
    }
}

#[pymethods]
impl Score {
    #[new]
    pub fn new(name: String, factor: f64) -> Result<Self, anyhow::Error> {
        Ok(Score {
            inner: auditor::domain::Score::new(name, factor)?,
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.as_ref().to_string()
    }

    #[getter]
    fn amount(&self) -> f64 {
        *self.inner.factor.as_ref()
    }
}

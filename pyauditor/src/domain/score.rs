// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use pyo3::prelude::*;

/// Score(name: str, factor: float)
/// An individual score which consists of a ``name`` (str) and a ``factor`` (float).
/// Scores are attached to a ``Component`` and are used to relate different components of the same
/// kind to each other in some kind of performance characteristic. For instance, in case of CPUs, a
/// score could be the corresponding HEPSPEC06 values.
///
/// The factor must be ``>= 0.0`` and the name must not include the characters. ``/``, ``(``, ``)``,
/// ``"``, ``<``, ``>``, ``\``, ``{``, ``}``.
///
/// :param name: Name of the score
/// :type name: str
/// :param factor: Factor
/// :type factor: float
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

    /// Returns the name
    #[getter]
    fn name(&self) -> String {
        self.inner.name.as_ref().to_string()
    }

    /// Returns the factor
    #[getter]
    fn factor(&self) -> f64 {
        *self.inner.factor.as_ref()
    }
}

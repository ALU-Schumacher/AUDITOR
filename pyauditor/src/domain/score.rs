// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use pyo3::IntoPyObjectExt;
use pyo3::class::basic::CompareOp;
use pyo3::prelude::*;

/// Score(name: str, value: float)
/// An individual score which consists of a ``name`` (str) and a ``value`` (float).
/// Scores are attached to a ``Component`` and are used to relate different components of the same
/// kind to each other in some kind of performance characteristic. For instance, in case of CPUs, a
/// score could be the corresponding HEPSPEC06 values.
///
/// The value must be ``>= 0.0`` and the name must not include the characters. ``/``, ``(``, ``)``,
/// ``"``, ``<``, ``>``, ``\``, ``{``, ``}``.
///
/// :param name: Name of the score
/// :type name: str
/// :param value: Value
/// :type value: float
#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn new(name: String, value: f64) -> Result<Self, anyhow::Error> {
        Ok(Score {
            inner: auditor::domain::Score::new(name, value)?,
        })
    }

    /// Returns the name
    #[getter]
    fn name(&self) -> String {
        self.inner.name.as_ref().to_string()
    }

    /// Returns the value
    #[getter]
    fn value(&self) -> f64 {
        *self.inner.value.as_ref()
    }

    fn __richcmp__(&self, other: PyRef<Score>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.inner == other.inner).into_py_any(py).unwrap(),
            CompareOp::Ne => (self.inner != other.inner).into_py_any(py).unwrap(),
            _ => py.NotImplemented(),
        }
    }
}

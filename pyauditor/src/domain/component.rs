// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Score;
use pyo3::IntoPyObjectExt;
use pyo3::class::basic::CompareOp;
use pyo3::prelude::*;

/// Component(name: str, amount: int)
/// A component represents a single component which is to be accounted for. It consists at least
/// of a ``name`` and an ``amount`` (how many or how much of this component is to be accounted
/// for).
/// Multiple scores can be attached to a single ``Component``.
///
/// The amount must be ``>= 0`` and the name must not include the characters. ``/``, ``(``, ``)``,
/// ``"``, ``<``, ``>``, ``\``, ``{``, ``}``.
///
/// :param name: Name of the component
/// :type name: str
/// :param amount: Amount
/// :type amount: int
#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Component {
    pub(crate) inner: auditor::domain::Component,
}

#[pymethods]
impl Component {
    #[new]
    pub fn new(name: String, amount: i64) -> Result<Self, anyhow::Error> {
        Ok(Component {
            inner: auditor::domain::Component::new(name, amount)?,
        })
    }

    /// with_score(score: Score)
    /// Attaches a score to the ``Component``.
    fn with_score(mut self_: PyRefMut<Self>, score: Score) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().with_score(score.inner);
        self_
    }

    /// Returns the name of the component
    #[getter]
    fn name(&self) -> String {
        self.inner.name.as_ref().to_string()
    }

    /// Returns the amount of the component
    #[getter]
    fn amount(&self) -> i64 {
        *self.inner.amount.as_ref()
    }

    /// Returns all scores attached to the component
    #[getter]
    fn scores(&self) -> Vec<Score> {
        self.inner.scores.iter().cloned().map(Score::from).collect()
    }

    fn __richcmp__(&self, other: PyRef<Component>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.inner == other.inner).into_py_any(py).unwrap(),
            CompareOp::Ne => (self.inner != other.inner).into_py_any(py).unwrap(),
            _ => py.NotImplemented(),
        }
    }
}

impl From<auditor::domain::Component> for Component {
    fn from(component: auditor::domain::Component) -> Component {
        Component { inner: component }
    }
}

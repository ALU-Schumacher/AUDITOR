// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use pyo3::IntoPyObjectExt;
use pyo3::class::basic::CompareOp;
use pyo3::prelude::*;

/// Meta()
///
/// Meta stores a list of key value pairs of the form `String -> [String]`.
///
/// The strings must not include the characters. ``/``, ``(``, ``)``,
/// ``"``, ``<``, ``>``, ``\``, ``{``, ``}``.
#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Meta {
    pub(crate) inner: auditor::domain::Meta,
}

#[pymethods]
impl Meta {
    #[new]
    pub fn new() -> Self {
        Meta {
            inner: auditor::domain::Meta::new(),
        }
    }

    /// insert(key: str, value: [str])
    /// Insert a key-value pair into Meta
    ///
    /// :param key: Key
    /// :type key: str
    /// :param value: Value
    /// :type value: [str]
    fn insert(mut self_: PyRefMut<Self>, key: String, value: Vec<String>) -> PyRefMut<Self> {
        self_.inner.insert(key, value);
        self_
    }

    /// get(key: str)
    /// Returns a list of string values matching the given key
    ///
    /// :param key: Key to get
    /// :type key: str
    fn get(&self, key: String) -> Option<Vec<String>> {
        self.inner.get(&key).cloned()
    }

    fn __richcmp__(&self, other: PyRef<Meta>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.inner == other.inner).into_py_any(py).unwrap(),
            CompareOp::Ne => (self.inner != other.inner).into_py_any(py).unwrap(),
            _ => py.NotImplemented(),
        }
    }
}

impl From<auditor::domain::Meta> for Meta {
    fn from(meta: auditor::domain::Meta) -> Meta {
        Meta { inner: meta }
    }
}

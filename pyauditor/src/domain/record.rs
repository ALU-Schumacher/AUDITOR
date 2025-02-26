// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(clippy::borrow_deref_ref)]

use crate::domain::{Component, Meta};
use anyhow::Error;
use auditor::domain::ValidName;
use chrono::{DateTime, Utc};
use pyo3::IntoPyObjectExt;
use pyo3::class::basic::CompareOp;
use pyo3::prelude::*;
use pyo3::types::PyDateTime;

/// Record(record_id: str, start_time: datetime.datetime)
/// A Record represents a single accountable unit. It consists of meta information such as
///
/// * ``record_id``: Uniquely identifies the record
/// * ``start_time``: Timestamp from when the resource was available.
///
/// .. note::
///    All strings must not include the characters. ``/``, ``(``, ``)``, ``"``, ``<``, ``>``, ``\``,
///    ``{``, ``}``.
///
/// .. warning::
///    All timestamps must be in UTC! Make sure to create time stamps in UTC or translate them to
///    UTC before using them in a ``Record``.
///
/// Records can be sent to and received from Auditor.
///
/// When created using the constructor for sending to Auditor, the record is already valid in terms
/// of all checks that Auditor performs when receiving it.
///
/// The optional ``stop_time`` can be added via the ``with_stop_time`` method.
///
/// Components are added via ``with_component``. Call this method multiple times for adding
/// multiple components.
///
/// Meta information is added via ``with_meta``.
///
/// The individual fields of the record can be accessed using the getter methods described below.
///
/// :param record_id: Unique record identifier
/// :type record_id: str
/// :param start_time: Timestamp from which the resource became available
/// :type group_id: datetime.datetime
#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    pub(crate) inner: auditor::domain::Record,
}

#[pymethods]
impl Record {
    #[new]
    fn new(record_id: String, start_time: &Bound<'_, PyDateTime>) -> Result<Self, Error> {
        let start_time: DateTime<Utc> = start_time.extract()?;
        Ok(Record {
            inner: auditor::domain::Record {
                record_id: ValidName::parse(record_id)?.as_ref().to_owned(),
                meta: None,
                components: Some(vec![]),
                start_time: Some(start_time),
                stop_time: None,
                runtime: None,
            },
        })
    }

    /// with_meta(meta: Meta)
    /// Adds Meta to the record.
    ///
    /// :param meta: Meta datastructure
    /// :type meta: Meta
    fn with_meta(mut self_: PyRefMut<Self>, meta: Meta) -> Result<PyRefMut<Self>, Error> {
        self_.inner.meta = Some(meta.inner);
        Ok(self_)
    }

    /// with_component(component: Component)
    /// Adds a component to the record. Use this method multiple times to attach multiple
    /// components.
    ///
    /// :param component: Component which is to be added
    /// :type component: Component
    fn with_component(
        mut self_: PyRefMut<Self>,
        component: Component,
    ) -> Result<PyRefMut<Self>, Error> {
        self_
            .inner
            .components
            .as_mut()
            .unwrap()
            .push(component.inner);
        Ok(self_)
    }

    /// with_stop_time(stop_time: datetime.datetime)
    /// Adds a stop_time to the record. This is the time when the resource stopped being available.
    ///
    /// :param stop_time: Timestamp when resource stopped being available
    /// :type stop_time: datetime.datetime
    fn with_stop_time<'a>(
        mut self_: PyRefMut<'a, Self>,
        stop_time: &Bound<'a, PyDateTime>,
    ) -> Result<PyRefMut<'a, Self>, Error> {
        let stop_time: DateTime<Utc> = stop_time.extract()?;
        self_.inner.stop_time = Some(stop_time);
        if let Some(start_time) = self_.inner.start_time.as_ref() {
            self_.inner.runtime = Some((stop_time - *start_time).num_seconds())
        }
        Ok(self_)
    }

    /// Returns the record_id
    #[getter]
    fn record_id(&self) -> String {
        self.inner.record_id.clone()
    }

    /// Returns the components
    ///
    /// Returns None if no components are attached, otherwise returns a list of ``Component`` s.
    #[getter]
    fn components(&self) -> Option<Vec<Component>> {
        self.inner
            .components
            .as_ref()
            .map(|components| components.iter().cloned().map(Component::from).collect())
    }

    /// Returns the meta object
    ///
    /// Returns None if no meta is available, otherwise returns a meta object.
    #[getter]
    fn meta(&self) -> Option<Meta> {
        self.inner.meta.clone().map(Meta::from)
    }

    /// Returns the start_time
    #[getter]
    fn start_time(&self, py: Python) -> Option<Py<PyAny>> {
        self.inner
            .start_time
            .as_ref()
            .map(|start_time| start_time.naive_utc().into_py_any(py).unwrap())
    }

    /// Returns the stop_time
    #[getter]
    fn stop_time(&self, py: Python) -> Option<Py<PyAny>> {
        self.inner
            .stop_time
            .as_ref()
            .map(|stop_time| stop_time.naive_utc().into_py_any(py).unwrap())
    }

    /// Returns the runtime of a record.
    #[getter]
    fn runtime(&self) -> Option<i64> {
        self.inner.runtime
    }

    /// Output content of Record as JSON-encoded string
    fn to_json(&self) -> Result<String, Error> {
        Ok(format!("{}", serde_json::to_value(&self.inner)?))
    }

    fn __richcmp__(&self, other: PyRef<Record>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.inner == other.inner).into_py_any(py).unwrap(),
            CompareOp::Ne => (self.inner != other.inner).into_py_any(py).unwrap(),
            _ => py.NotImplemented(),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

impl From<auditor::domain::Record> for Record {
    fn from(record: auditor::domain::Record) -> Record {
        Record { inner: record }
    }
}

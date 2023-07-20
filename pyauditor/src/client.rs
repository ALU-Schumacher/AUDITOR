// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Record;
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::PyDateTime;

/// The `AuditorClient` handles the interaction with the Auditor instances and allows one to add
/// records to the database, update records in the database and retrieve the records from the
/// database.
#[pyclass]
#[derive(Clone)]
pub struct AuditorClient {
    pub(crate) inner: auditor::client::AuditorClient,
}

#[pymethods]
impl AuditorClient {
    /// health_check()
    /// Returns ``true`` if the Auditor instance is healthy, ``false`` otherwise
    fn health_check<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner.health_check().await)
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    /// get()
    /// Gets all records from the Auditors database
    fn get<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get()
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
        })
    }

    /// get_started_since(timestamp: datetime.datetime)
    /// Get all records in the database with a started timestamp after ``timestamp``.
    ///
    /// .. warning::
    ///    The ``timestamp`` MUST be in UTC!
    ///
    /// :param timestamp: Timestamp in UTC
    /// :type timestamp: datetime.datetime
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     # If the date/time is already in UTC:
    ///     start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc)
    ///
    ///     # If it is in local time:
    ///     from tzlocal import get_localzone
    ///     local_tz = get_localzone()
    ///     start_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=local_tz).astimezone(datetime.timezone.utc)
    ///
    ///     records = client.get_stopped_since(start_since)
    ///
    fn get_started_since<'a>(
        self_: PyRef<'a, Self>,
        timestamp: &PyDateTime,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let timestamp: DateTime<Utc> = timestamp.extract()?;
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get_started_since(&timestamp)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
        })
    }

    /// get_stopped_since(timestamp: datetime.datetime)
    /// Get all records in the database with a stopped timestamp after ``timestamp``.
    ///
    /// .. warning::
    ///    The ``timestamp`` MUST be in UTC!
    ///
    /// :param timestamp: Timestamp in UTC
    /// :type timestamp: datetime.datetime
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     # If the date/time is already in UTC:
    ///     stop_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc)
    ///
    ///     # If it is in local time:
    ///     from tzlocal import get_localzone
    ///     local_tz = get_localzone()
    ///     stop_since = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=local_tz).astimezone(datetime.timezone.utc)
    ///
    ///     records = client.get_stopped_since(stop_since)
    ///
    fn get_stopped_since<'a>(
        self_: PyRef<'a, Self>,
        timestamp: &PyDateTime,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let timestamp: DateTime<Utc> = timestamp.extract()?;
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get_stopped_since(&timestamp)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
        })
    }

    /// add(record: Record)
    /// Push a record to the Auditor instance
    fn add<'a>(&self, record: Record, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            inner
                .add(&auditor::domain::RecordAdd::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }

    /// update(record: Record)
    /// Update an existing record in the Auditor instance
    fn update<'a>(&self, record: Record, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            inner
                .update(&auditor::domain::RecordUpdate::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }
}
// Ok(Python::with_gil(|py| py.None()))

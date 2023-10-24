// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Record;
use anyhow::Error;
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::PyDateTime;

#[pyclass]
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub(crate) inner: auditor::client::QueryBuilder,
}

#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct TimeOperator {
    pub(crate) inner: auditor::client::TimeOperator,
}

#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TimeValue {
    pub(crate) inner: auditor::client::TimeValue,
}

#[pymethods]
impl TimeValue {
    #[staticmethod]
    fn set_datetime(datetime: &PyDateTime) -> Result<Self, Error> {
        let date_time: DateTime<Utc> = datetime.extract()?;
        Ok(TimeValue {
            inner: auditor::client::TimeValue::Datetime(auditor::client::DateTimeUtcWrapper(
                date_time,
            )),
        })
    }

    #[staticmethod]
    fn runtime(runtime: u64) -> Result<Self, Error> {
        Ok(TimeValue {
            inner: auditor::client::TimeValue::Runtime(runtime),
        })
    }
}

#[pymethods]
impl TimeOperator {
    #[new]
    fn new() -> Self {
        TimeOperator {
            inner: auditor::client::TimeOperator {
                gt: None,
                gte: None,
                lt: None,
                lte: None,
            },
        }
    }

    fn gt(mut self_: PyRefMut<Self>, value: TimeValue) -> PyRefMut<Self> {
        self_.inner.gt = Some(value.inner);
        self_
    }

    fn lt(mut self_: PyRefMut<Self>, value: TimeValue) -> PyRefMut<Self> {
        self_.inner.lt = Some(value.inner);
        self_
    }

    fn gte(mut self_: PyRefMut<Self>, value: TimeValue) -> PyRefMut<Self> {
        self_.inner.gte = Some(value.inner);
        self_
    }

    fn lte(mut self_: PyRefMut<Self>, value: TimeValue) -> PyRefMut<Self> {
        self_.inner.lte = Some(value.inner);
        self_
    }
}

#[pymethods]
impl QueryBuilder {
    #[new]
    fn new() -> Result<Self, Error> {
        Ok(QueryBuilder {
            inner: auditor::client::QueryBuilder {
                query_params: auditor::client::QueryParameters {
                    start_time: None,
                    stop_time: None,
                    runtime: None,
                },
            },
        })
    }

    fn with_start_time(
        mut self_: PyRefMut<Self>,
        operator: TimeOperator,
    ) -> Result<PyRefMut<Self>, Error> {
        self_.inner.query_params.start_time = Some(operator.inner);
        Ok(self_)
    }

    fn with_stop_time(mut self_: PyRefMut<Self>, operator: TimeOperator) -> PyRefMut<Self> {
        self_.inner.query_params.stop_time = Some(operator.inner);
        self_
    }

    fn with_runtime(mut self_: PyRefMut<Self>, operator: TimeOperator) -> PyRefMut<Self> {
        self_.inner.query_params.runtime = Some(operator.inner);
        self_
    }

    fn build(self_: PyRef<Self>, py: Python) -> Py<PyAny> {
        let query_string: String = self_.inner.clone().build();
        query_string.into_py(py)
    }
}

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

    fn advanced_query<'a>(
        self_: PyRef<'a, Self>,
        query_string: String,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .advanced_query(query_string)
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

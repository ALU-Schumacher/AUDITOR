// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Record;
use chrono::{DateTime, Utc};
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDateTime;

/// The `QueuedAuditorClient` handles the interaction with the Auditor instances. All
/// data to be sent is transparently saved in a persistent local database.
///
/// When records are sent to Auditor, this client will transparently buffer them in a
/// (persistent) local database.
/// A background task will then periodically send records from the local database to
/// Auditor, deleting them from the local database only after they have been successfully
/// send to Auditor.
///
/// .. note::
///    There are some quirks that need to be observed when using this client:
///     - Uses an in-memory database by default. It is strongly
///       recommended to provide a path.
///       See :meth:`AuditorClientBuilder.database_path`.
///     - Since sending and updating records is delayed, there is no guarantee that a record
///       can be retrieved from Auditor right after it has been "sent" by this client.
///     - The background task of this client should be stopped by invoking
///       :meth:`QueuedAuditorClient.stop`
///       before the client is dropped.

#[pyclass]
#[derive(Clone)]
pub struct QueuedAuditorClient {
    pub(crate) inner: auditor_client::QueuedAuditorClient,
}

#[pymethods]
impl QueuedAuditorClient {
    /// health_check()
    /// Returns ``true`` if the Auditor instance is healthy, ``false`` otherwise
    fn health_check<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self_.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(inner.health_check().await)
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    /// get()
    /// Gets all records from the Auditors database
    fn get<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self_.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
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
        timestamp: &Bound<'_, PyDateTime>,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let message = py.get_type::<pyo3::exceptions::PyDeprecationWarning>();
        PyErr::warn(py, &message, c_str!("get_started_since is depreciated"), 0)?;

        let since: DateTime<Utc> = timestamp.extract()?;
        let inner = self_.inner.clone();
        let query_string = auditor_client::QueryBuilder::new()
            .with_start_time(auditor_client::Operator::default().gte(since.into()))
            .build();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(inner
                .advanced_query(query_string.to_string())
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
        timestamp: &Bound<'_, PyDateTime>,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let message = py.get_type::<pyo3::exceptions::PyDeprecationWarning>();
        PyErr::warn(py, &message, c_str!("get_stopped_since is depreciated"), 0)?;

        let since: DateTime<Utc> = timestamp.extract()?;
        let inner = self_.inner.clone();
        let query_string = auditor_client::QueryBuilder::new()
            .with_stop_time(auditor_client::Operator::default().gte(since.into()))
            .build();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(inner
                .advanced_query(query_string.to_string())
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
        })
    }

    /// advanced_query(query_string: string)
    /// Get records from the database depending on the query parameters
    ///
    /// :param query_string: query_string constructed with QueryBuilder using .build() method
    /// :type query_string: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     value1 = Value.set_datetime(start_time)
    ///     value2 = Value.set_datetime(stop_time)
    ///     operator1 = Operator().gt(value1)
    ///     operator2 = Operator().gt(value2)
    ///     query_string = QueryBuilder().with_start_time(operator1).with_stop_time(operator2).build()
    ///     records = await client.advanced_query(query_string)
    fn advanced_query<'a>(
        self_: PyRef<'a, Self>,
        query_string: String,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = self_.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            Ok(inner
                .advanced_query(query_string)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
        })
    }

    /// get_one_record(record_id: string)
    /// Get one record using record_id
    ///
    /// :param record_id: record_id
    /// :type record_id: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     record: &str = "record-1"
    ///     record = await client.get_one_record(record)
    fn get_single_record<'a>(
        self_: PyRef<'a, Self>,
        record_id: String,
        py: Python<'a>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = self_.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner
                .get_single_record(record_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
                .map(Record::from)
        })
    }

    /// add(record: Record)
    /// Push a record to the Auditor instance
    fn add<'a>(&self, record: Record, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner
                .add(&auditor::domain::RecordAdd::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }

    /// add(record: Record)
    /// Push a list of records to the Auditor instance
    fn bulk_insert<'a>(&self, records: Vec<Record>, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();

        let bulk_insert_records: Result<Vec<auditor::domain::RecordAdd>, anyhow::Error> = records
            .into_iter()
            .map(|r| auditor::domain::RecordAdd::try_from(r.inner))
            .collect();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let bul = bulk_insert_records?;
            inner
                .bulk_insert(&bul)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }

    /// update(record: Record)
    /// Update an existing record in the Auditor instance
    fn update<'a>(&self, record: Record, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner
                .update(&auditor::domain::RecordUpdate::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }

    /// Stops the background sync task
    fn stop<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let mut inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner
                .stop()
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
        })
    }
}
// Ok(Python::with_gil(|py| py.None()))

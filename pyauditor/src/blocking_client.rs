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

/// The `AuditorClientBlocking` handles the interaction with the Auditor instances and allows one to add
/// records to the database, update records in the database and retrieve the records from the
/// database. In contrast to `AuditorClient`, no async runtime is needed here.
#[pyclass]
#[derive(Clone)]
pub struct AuditorClientBlocking {
    pub(crate) inner: auditor_client::AuditorClientBlocking,
}

#[pymethods]
impl AuditorClientBlocking {
    /// health_check()
    /// Returns ``true`` if the Auditor instance is healthy, ``false`` otherwise
    fn health_check(self_: PyRef<'_, Self>) -> bool {
        self_.inner.health_check()
    }

    /// get()
    /// Gets all records from the Auditors database
    fn get(self_: PyRef<'_, Self>) -> PyResult<Vec<Record>> {
        Ok(self_
            .inner
            .get()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
            .into_iter()
            .map(Record::from)
            .collect::<Vec<_>>())
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
    fn get_started_since(
        self_: PyRef<'_, Self>,
        timestamp: &Bound<'_, PyDateTime>,
    ) -> PyResult<Vec<Record>> {
        let since: DateTime<Utc> = timestamp.extract()?;
        let query_string = auditor_client::QueryBuilder::new()
            .with_start_time(auditor_client::Operator::default().gte(since.into()))
            .build();
        Ok(self_
            .inner
            .advanced_query(query_string.to_string())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
            .into_iter()
            .map(Record::from)
            .collect::<Vec<_>>())
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
    fn get_stopped_since(
        self_: PyRef<'_, Self>,
        timestamp: &Bound<'_, PyDateTime>,
    ) -> PyResult<Vec<Record>> {
        let since: DateTime<Utc> = timestamp.extract()?;
        let query_string = auditor_client::QueryBuilder::new()
            .with_stop_time(auditor_client::Operator::default().gte(since.into()))
            .build();
        Ok(self_
            .inner
            .advanced_query(query_string.to_string())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
            .into_iter()
            .map(Record::from)
            .collect::<Vec<_>>())
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
    ///     records = client.advanced_query(query_string)
    fn advanced_query(self_: PyRef<'_, Self>, query_string: String) -> PyResult<Vec<Record>> {
        Ok(self_
            .inner
            .advanced_query(query_string)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?
            .into_iter()
            .map(Record::from)
            .collect::<Vec<_>>())
    }

    /// add(record: Record)
    /// Push a record to the Auditor instance
    fn add(&self, record: Record) -> PyResult<()> {
        self.inner
            .add(&auditor::domain::RecordAdd::try_from(record.inner)?)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
    }

    /// add(record: Record)
    /// Push a list of records to the Auditor instance
    fn bulk_insert(&self, records: Vec<Record>) -> PyResult<()> {
        let bulk_insert_records: Result<Vec<auditor::domain::RecordAdd>, anyhow::Error> = records
            .into_iter()
            .map(|r| auditor::domain::RecordAdd::try_from(r.inner))
            .collect();

        self.inner
            .bulk_insert(&bulk_insert_records?)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
    }

    /// update(record: Record)
    /// Update an existing record in the Auditor instance
    fn update(&self, record: Record) -> PyResult<()> {
        self.inner
            .update(&auditor::domain::RecordUpdate::try_from(record.inner)?)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
    }

    fn get_single_record(self_: PyRef<'_, Self>, record_id: String) -> PyResult<Record> {
        self_
            .inner
            .get_single_record(&record_id)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))
            .map(Record::from)
    }
}

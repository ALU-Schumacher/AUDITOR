// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::Record;
use anyhow::Error;
use chrono::{DateTime, Utc};
use pyo3::IntoPyObjectExt;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDateTime;
use std::collections::HashMap;

/// The `QueryBuilder` is used to construct `QueryParameters` using the builder pattern.
#[pyclass]
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub(crate) inner: auditor_client::QueryBuilder,
}

/// The `Operator` is used to specify the operators on the query parameters
#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct Operator {
    pub(crate) inner: auditor_client::Operator,
}

/// Value is used to specify the type of the value which is used in the query
#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Value {
    pub(crate) inner: auditor_client::Value,
}

/// Creates `Value` object which is passed to set the operator value
#[pymethods]
impl Value {
    /// Sets datetime for start_time and stop_time queries
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
    ///     start_time = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc)
    ///
    ///     # If it is in local time:
    ///     from tzlocal import get_localzone
    ///     local_tz = get_localzone()
    ///     start_time = datetime.datetime(2022, 8, 8, 11, 30, 0, 0, tzinfo=local_tz).astimezone(datetime.timezone.utc)
    ///
    ///     value = Value.set_datetime(start_time)
    #[staticmethod]
    fn set_datetime(datetime: &Bound<'_, PyDateTime>) -> Result<Self, Error> {
        let date_time: DateTime<Utc> = datetime.extract()?;
        Ok(Value {
            inner: auditor_client::Value::Datetime(auditor_client::DateTimeUtcWrapper(date_time)),
        })
    }

    /// Sets the runtime value to query
    ///
    /// :param runtime: int
    /// :type runtime: int
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     runtime_value = 100000
    ///     value = Value.set_runtime(runtime_value)
    #[staticmethod]
    fn set_runtime(runtime: u64) -> Result<Self, Error> {
        Ok(Value {
            inner: auditor_client::Value::Runtime(runtime),
        })
    }

    /// Sets the count value
    /// Sets the runtime value to query
    ///
    /// :param count: int
    /// :type count: int
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    #[staticmethod]
    fn set_count(count: u8) -> Result<Self, Error> {
        Ok(Value {
            inner: auditor_client::Value::Count(count),
        })
    }
}

#[pymethods]
impl Operator {
    /// Constructor for setting the operator value
    #[new]
    fn new() -> Self {
        Operator {
            inner: auditor_client::Operator {
                gt: None,
                gte: None,
                lt: None,
                lte: None,
                equals: None,
            },
        }
    }

    /// Sets the greater than (`gt`) operator Value
    ///
    /// :param value: The value for greater than operator
    /// :type value: `Value` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    ///     operator = Operator().gt(value)
    fn gt(mut self_: PyRefMut<Self>, value: Value) -> PyRefMut<Self> {
        self_.inner.gt = Some(value.inner);
        self_
    }

    /// Sets the lesser than (`lt`) operator value
    ///
    /// :param value: The value for lesser than operator
    /// :type value: `Value` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    ///     operator = Operator().lt(value)
    fn lt(mut self_: PyRefMut<Self>, value: Value) -> PyRefMut<Self> {
        self_.inner.lt = Some(value.inner);
        self_
    }

    /// Sets the greater than or equal to (`gte`) operator value
    ///
    /// :param value: The value for greater than or equal to operator
    /// :type value: `Value` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    ///     operator = Operator().gte(value)
    fn gte(mut self_: PyRefMut<Self>, value: Value) -> PyRefMut<Self> {
        self_.inner.gte = Some(value.inner);
        self_
    }

    /// Sets lesser than or equal to (`lte`) operator value
    ///
    /// :param value: The value for greater than operator
    /// :type value: `Value` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    ///     operator = Operator().lte(value)
    fn lte(mut self_: PyRefMut<Self>, value: Value) -> PyRefMut<Self> {
        self_.inner.lte = Some(value.inner);
        self_
    }

    /// Sets equal to (`equals`) operator value
    ///
    /// :param value: The value for greater than operator
    /// :type value: `Value` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     count_value = 100000
    ///     value = Value.set_count(count_value)
    ///     operator = Operator().equals(value)
    fn equals(mut self_: PyRefMut<Self>, value: Value) -> PyRefMut<Self> {
        self_.inner.equals = Some(value.inner);
        self_
    }
}

#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MetaQuery {
    pub(crate) inner: auditor_client::MetaQuery,
}

#[pymethods]
impl MetaQuery {
    /// Constructor for MetaQuery object
    #[new]
    fn new() -> Self {
        MetaQuery {
            inner: auditor_client::MetaQuery {
                meta_query: HashMap::new(),
            },
        }
    }

    /// Sets meta operator Value
    ///
    /// :param query_id: Metadata key
    /// :type query_id: string
    ///
    /// :param meta_operator: Metadata value which is set using MetaOperator object
    /// :type meta_operator: `MetaOperator` object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     meta_operator = MetaOperator().contains("group_1")
    ///     operator = MetaQuery().meta_operator("group_id", meta_operator)
    fn meta_operator(
        mut self_: PyRefMut<Self>,
        query_id: String,
        meta_operator: MetaOperator,
    ) -> PyRefMut<Self> {
        self_
            .inner
            .meta_query
            .insert(query_id, Some(meta_operator.inner));
        self_
    }
}

/// The `MetaOperator` struct represents operators for metadata queries, specifying conditions for filtering
#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MetaOperator {
    pub(crate) inner: auditor_client::MetaOperator,
}

#[pymethods]
impl MetaOperator {
    /// Constructor for MetaOperator object
    #[new]
    fn new() -> Self {
        MetaOperator {
            inner: auditor_client::MetaOperator { c: None, dnc: None },
        }
    }

    /// Sets the meta value using contains operator. This checks if the value exists for the
    /// corresponding metadata key
    ///
    /// :param query_id: Metadata key
    /// :type query_id: string
    ///
    /// :param c: Metadata value to be checked if it exists
    /// :type c: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     meta_operator = MetaOperator().contains("group_1")
    ///     meta_query = MetaQuery().meta_operator("group_id", meta_operator)
    fn contains(mut self_: PyRefMut<Self>, c: Vec<String>) -> PyRefMut<Self> {
        self_.inner.c = Some(c);
        self_
    }

    /// Sets the meta value using does not contain operator. This checks if the value does not exist for the
    /// corresponding metadata key
    ///
    /// :param query_id: Metadata key
    /// :type query_id: string
    ///
    /// :param c: Metadata value to be checked if it does not exist
    /// :type c: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     operator = MetaOperator().does_not_contain("group_1")
    fn does_not_contain(mut self_: PyRefMut<Self>, dnc: Vec<String>) -> PyRefMut<Self> {
        self_.inner.dnc = Some(dnc);
        self_
    }
}

/// The `ComponentQuery` struct represents a set of component queries associated with specific query IDs.
/// It is used to filter records based on component-related conditions
#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ComponentQuery {
    pub(crate) inner: auditor_client::ComponentQuery,
}

#[pymethods]
impl ComponentQuery {
    /// Constructor for ComponentQuery object
    #[new]
    fn new() -> Self {
        ComponentQuery {
            inner: auditor_client::ComponentQuery {
                component_query: HashMap::new(),
            },
        }
    }

    /// Adds a new component operator to the `ComponentQuery` instance for a specific query ID.
    ///
    /// :param query_id: Component name
    /// :type query_id: string
    ///
    /// :param operator: component amount
    /// :type operator: int
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     value = Value.set_count(10)
    ///     component_operator = Operator().equals(value)
    ///     component_query = ComponentQuery().component_operator("cpu", component_operator)
    fn component_operator(
        mut self_: PyRefMut<Self>,
        query_id: String,
        operator: Operator,
    ) -> PyRefMut<Self> {
        self_
            .inner
            .component_query
            .insert(query_id, Some(operator.inner));
        self_
    }
}

/// SortBy provides options on sorting the query records
#[pyclass]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SortBy {
    pub(crate) inner: auditor_client::SortBy,
}

#[pymethods]
impl SortBy {
    /// Constructor for SortBy object
    #[new]
    fn new() -> Self {
        Self {
            inner: auditor_client::SortBy {
                asc: None,
                desc: None,
            },
        }
    }

    /// Specify the column by which the query records must be sorted in ascending order
    ///
    /// :param column: Name of the column by which the records must be sorted. One of four values (`start_time`, `stop_time`, `runtime`, `record_id`).
    /// :type column: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     sort_by = SortBy().ascending("start_time")
    fn ascending(mut self_: PyRefMut<Self>, column: String) -> PyRefMut<Self> {
        self_.inner.asc = Some(column);
        self_
    }

    /// Specify the column by which the query records must be sorted in descending order
    ///
    /// :param column: Name of the column by which the records must be sorted. One of three values (`start_time`, `stop_time`, `runtime`, `record_id`).
    /// :type column: string
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     sort_by = SortBy().descending("start_time")
    fn descending(mut self_: PyRefMut<Self>, column: String) -> PyRefMut<Self> {
        self_.inner.desc = Some(column);
        self_
    }
}

#[pymethods]
impl QueryBuilder {
    /// Constructor for QueryBuilder object
    #[new]
    fn new() -> Result<Self, Error> {
        Ok(QueryBuilder {
            inner: auditor_client::QueryBuilder {
                query_params: auditor_client::QueryParameters {
                    record_id: None,
                    start_time: None,
                    stop_time: None,
                    runtime: None,
                    meta: None,
                    component: None,
                    sort_by: None,
                    limit: None,
                },
            },
        })
    }

    /// Sets the exact record_id to retrieve
    ///
    /// :param record_id: Exact record_id to be retrieved
    /// :type record_id: string
    ///
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     record_id = "r101"
    ///     query_string = QueryBuilder().with_record_id(record_id).build()
    fn with_record_id(
        mut self_: PyRefMut<Self>,
        record_id: String,
    ) -> Result<PyRefMut<Self>, Error> {
        self_.inner.query_params.record_id = Some(record_id);
        Ok(self_)
    }

    /// Sets the start time in the query parameters
    ///
    /// :param operator: Operator object containing `DateTime<Utc>`
    /// :type operator: Operator object
    ///
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     start_time = datetime.datetime(
    ///      2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    ///     )
    ///     
    ///     value = Value.set_datetime(start_time)
    ///     operator = Operator().gt(value)
    ///     query_string = QueryBuilder().with_start_time(operator).build()
    fn with_start_time(
        mut self_: PyRefMut<Self>,
        operator: Operator,
    ) -> Result<PyRefMut<Self>, Error> {
        self_.inner.query_params.start_time = Some(operator.inner);
        Ok(self_)
    }

    /// Sets the stop time in the query parameters
    ///
    /// :param operator: Operator object containing `DateTime<Utc>`
    /// :type operator: Operator object
    ///
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     stop_time = datetime.datetime(
    ///      2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    ///     )
    ///     
    ///     value = Value.set_datetime(stop_time)
    ///     operator = Operator().gt(value)
    ///     query_string = QueryBuilder().with_stop_time(operator).build()
    fn with_stop_time(mut self_: PyRefMut<Self>, operator: Operator) -> PyRefMut<Self> {
        self_.inner.query_params.stop_time = Some(operator.inner);
        self_
    }

    /// Sets the runtime in the query parameters
    ///
    /// :param operator: Operator object containing runtime value
    /// :type operator: Operator object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     start_time = datetime.datetime(
    ///      2022, 8, 8, 11, 30, 0, 0, tzinfo=datetime.timezone.utc
    ///     )
    ///      
    ///     value = Value.set_runtime(100000)
    ///     operator = Operator().gt(value)
    ///     query_string = QueryBuilder().with_runtime(operator).build()
    fn with_runtime(mut self_: PyRefMut<Self>, operator: Operator) -> PyRefMut<Self> {
        self_.inner.query_params.runtime = Some(operator.inner);
        self_
    }

    /// Sets the meta query in the query parameters
    ///
    /// :param meta- meta contains the meta key and value
    /// :type meta- MetaQuery object
    ///
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///     
    ///     meta_operator = MetaOperator().contains("group_1")
    ///     meta_query = MetaQuery().meta_operator("group_id", meta_operator)
    ///     query_string = QueryBuilder().with_meta_query(meta_query).build()
    fn with_meta_query(mut self_: PyRefMut<Self>, meta: MetaQuery) -> PyRefMut<Self> {
        self_.inner.query_params.meta = Some(meta.inner);
        self_
    }

    /// Sets the component query in the query parameters
    ///
    /// :param component: ComponentQuery object instantiated with component name and amount
    /// :type component: ComponentQuery object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     value = Value.count(4)
    ///     component_operator = Operator().equals(value)
    ///     component_query = ComponentQuery().component_operator("cpu", component_operator)
    ///     records = QueryBuilder().with_component_query(component_query).build()
    fn with_component_query(
        mut self_: PyRefMut<Self>,
        component: ComponentQuery,
    ) -> PyRefMut<Self> {
        self_.inner.query_params.component = Some(component.inner);
        self_
    }

    /// SortBy provides options on sorting the query records
    ///
    /// :param sort_by: SortBy object instantiatied with the sorting order(asc or desc) and column
    /// name
    /// :type sort_by: SortBy object
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     sort_by = SortBy().ascending("start_time")
    ///     records = QueryBuilder().sort_by(sort_by).build()
    fn sort_by(mut self_: PyRefMut<Self>, sort_by: SortBy) -> PyRefMut<Self> {
        self_.inner.query_params.sort_by = Some(sort_by.inner);
        self_
    }

    /// Limits the query records
    ///
    /// :param number: number specifying the number of records that needs to be returned
    /// :type number: int
    ///
    /// **Example**
    ///
    /// .. code-block:: python
    ///
    ///     records = QueryBuilder().limit(500).build()
    fn limit(mut self_: PyRefMut<Self>, number: u64) -> PyRefMut<Self> {
        self_.inner.query_params.limit = Some(number);
        self_
    }

    /// Builds the query string for the given query parameters
    fn build(self_: PyRef<Self>, py: Python) -> Py<PyAny> {
        let query_string: String = self_.inner.clone().build();
        query_string.into_py_any(py).unwrap()
    }
}

/// The `AuditorClient` handles the interaction with the Auditor instances and allows one to add
/// records to the database, update records in the database and retrieve the records from the
/// database.
#[pyclass]
#[derive(Clone)]
pub struct AuditorClient {
    pub(crate) inner: auditor_client::AuditorClient,
}

#[pymethods]
impl AuditorClient {
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
}
// Ok(Python::with_gil(|py| py.None()))

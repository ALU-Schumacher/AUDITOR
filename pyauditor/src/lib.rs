// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(clippy::borrow_deref_ref)]

mod domain;

use crate::domain::{Component, Record, Score};
use anyhow::Error;
use chrono::{offset::TimeZone, Utc};
use pyo3::prelude::*;
use pyo3::types::PyDateTime;
use pyo3_chrono::NaiveDateTime;

/// A Python module implemented in Rust.
#[pymodule]
fn pyauditor(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AuditorClient>()?;
    m.add_class::<AuditorClientBuilder>()?;
    m.add_class::<Record>()?;
    m.add_class::<Component>()?;
    m.add_class::<Score>()?;
    Ok(())
}

#[pyclass]
#[derive(Clone)]
pub struct AuditorClient {
    inner: auditor::client::AuditorClient,
}

#[pyclass]
#[derive(Clone)]
pub struct AuditorClientBuilder {
    inner: auditor::client::AuditorClientBuilder,
}

impl Default for AuditorClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl AuditorClientBuilder {
    #[new]
    pub fn new() -> Self {
        AuditorClientBuilder {
            inner: auditor::client::AuditorClientBuilder::new(),
        }
    }

    pub fn build(&self) -> Result<AuditorClient, Error> {
        Ok(AuditorClient {
            // Must clone here because `build` moves the builder, but python
            // does not allow that. Doesn't matter, Python is slow anyways.
            inner: self.inner.clone().build()?,
        })
    }

    pub fn address(mut self_: PyRefMut<Self>, address: String, port: u16) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().address(&address, port);
        self_
    }

    pub fn connection_string(
        mut self_: PyRefMut<Self>,
        connection_string: String,
    ) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().connection_string(&connection_string);
        self_
    }

    pub fn timeout(mut self_: PyRefMut<Self>, timeout: i64) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().timeout(timeout);
        self_
    }
}

#[pymethods]
impl AuditorClient {
    fn health_check<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner.health_check().await)
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    fn get<'a>(self_: PyRef<'a, Self>, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get()
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{}", e)))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    fn get_started_since<'a>(
        self_: PyRef<'a, Self>,
        timestamp: &PyDateTime,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let timestamp: NaiveDateTime = timestamp.extract()?;
        let timestamp = Utc.from_utc_datetime(&timestamp.into());
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get_started_since(&timestamp)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{}", e)))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    fn get_stopped_since<'a>(
        self_: PyRef<'a, Self>,
        timestamp: &PyDateTime,
        py: Python<'a>,
    ) -> PyResult<&'a PyAny> {
        let timestamp: NaiveDateTime = timestamp.extract()?;
        let timestamp = Utc.from_utc_datetime(&timestamp.into());
        let inner = self_.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            Ok(inner
                .get_stopped_since(&timestamp)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{}", e)))?
                .into_iter()
                .map(Record::from)
                .collect::<Vec<_>>())
            // Ok(Python::with_gil(|py| py.None()))
        })
    }

    fn add<'a>(&self, record: Record, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            inner
                .add(&auditor::domain::RecordAdd::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{}", e)))
        })
    }

    fn update<'a>(&self, record: Record, py: Python<'a>) -> PyResult<&'a PyAny> {
        let inner = self.inner.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            inner
                .update(&auditor::domain::RecordUpdate::try_from(record.inner)?)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{}", e)))
        })
    }
}
// Ok(Python::with_gil(|py| py.None()))

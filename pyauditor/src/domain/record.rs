// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![allow(clippy::borrow_deref_ref)]

use crate::domain::Component;
use anyhow::Error;
use auditor::domain::ValidName;
use chrono::{offset::TimeZone, Local, Utc};
use pyo3::prelude::*;
use pyo3::types::PyDateTime;
use pyo3_chrono::NaiveDateTime;

#[pyclass]
#[derive(Clone)]
pub struct Record {
    inner: auditor::domain::Record,
}

#[pymethods]
impl Record {
    #[new]
    fn new(
        record_id: String,
        site_id: String,
        user_id: String,
        group_id: String,
        start_time: &PyDateTime,
    ) -> Result<Self, Error> {
        let start_time: NaiveDateTime = start_time.extract()?;
        let start_time = Local
            .from_local_datetime(&start_time.into())
            .unwrap()
            .with_timezone(&Utc);
        Ok(Record {
            inner: auditor::domain::Record {
                record_id: ValidName::parse(record_id)?.as_ref().to_owned(),
                site_id: Some(ValidName::parse(site_id)?.as_ref().to_owned()),
                user_id: Some(ValidName::parse(user_id)?.as_ref().to_owned()),
                group_id: Some(ValidName::parse(group_id)?.as_ref().to_owned()),
                components: Some(vec![]),
                start_time,
                stop_time: None,
                runtime: None,
            },
        })
    }

    #[getter]
    fn record_id(&self) -> String {
        self.inner.record_id.clone()
    }

    #[getter]
    fn site_id(&self) -> Option<String> {
        self.inner.site_id.clone()
    }

    #[getter]
    fn user_id(&self) -> Option<String> {
        self.inner.user_id.clone()
    }

    #[getter]
    fn group_id(&self) -> Option<String> {
        self.inner.group_id.clone()
    }

    #[getter]
    fn components(&self) -> Option<Vec<Component>> {
        self.inner
            .components
            .as_ref()
            .map(|components| components.iter().cloned().map(Component::from).collect())
    }

    #[getter]
    fn start_time(&self, py: Python) -> Py<PyAny> {
        NaiveDateTime(self.inner.start_time.naive_utc()).into_py(py)
    }

    #[getter]
    fn stop_time(&self, py: Python) -> Option<Py<PyAny>> {
        self.inner
            .stop_time
            .as_ref()
            .map(|stop_time| NaiveDateTime(stop_time.naive_utc()).into_py(py))
    }

    #[getter]
    fn runtime(&self) -> Option<i64> {
        self.inner.runtime
    }
}

impl From<auditor::domain::Record> for Record {
    fn from(record: auditor::domain::Record) -> Record {
        Record { inner: record }
    }
}

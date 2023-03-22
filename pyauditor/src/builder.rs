// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::{blocking_client::AuditorClientBlocking, client::AuditorClient};
use anyhow::Error;
use pyo3::prelude::*;

/// The ``AuditorClientBuilder`` class is used to build an instance of ``AuditorClient``.
///
/// **Examples**
///
/// Using the ``address`` and ``port`` of the Auditor instance:
///
/// .. code-block:: python
///
///     # Create an instance of the builder
///     builder = AuditorClientBuilder()
///
///     # Configure the builder
///     builder = builder.address("localhost", 8000).timeout(20)
///
///     # Build the client
///     client = builder.build()
///
///
/// Using an connection string:
///
/// .. code-block:: python
///
///     client = AuditorClientBuilder().connection_string("http://localhost:8000").build()
///
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
    /// Constructor
    #[new]
    pub fn new() -> Self {
        AuditorClientBuilder {
            inner: auditor::client::AuditorClientBuilder::new(),
        }
    }

    /// address(address: str, port: int)
    /// Set the address of the Auditor server
    ///
    /// :param address: Host name / IP address of the Auditor instance
    /// :type address: str
    /// :param port: Port of the Auditor instance
    /// :type port: int
    pub fn address(mut self_: PyRefMut<Self>, address: String, port: u16) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().address(&address, port);
        self_
    }

    /// connection_string(connection_string: str)
    /// Set a connection string of the form ``http://<auditor_address>:<auditor_port>``
    ///
    /// :param connection_string: Connection string
    /// :type connection_string: str
    pub fn connection_string(
        mut self_: PyRefMut<Self>,
        connection_string: String,
    ) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().connection_string(&connection_string);
        self_
    }

    /// timeout(timeout: int)
    /// Set a timeout in seconds for HTTP requests
    ///
    /// :param timeout: Timeout in sections
    /// :type timeout: int
    pub fn timeout(mut self_: PyRefMut<Self>, timeout: i64) -> PyRefMut<Self> {
        self_.inner = self_.inner.clone().timeout(timeout);
        self_
    }

    /// Build an ``AuditorClient`` from ``AuditorClientBuilder``
    pub fn build(&self) -> Result<AuditorClient, Error> {
        Ok(AuditorClient {
            // Must clone here because `build` moves the builder, but python
            // does not allow that. Doesn't matter, Python is slow anyways.
            inner: self.inner.clone().build()?,
        })
    }

    /// Build an ``AuditorClientBlocking`` from ``AuditorClientBuilder``
    pub fn build_blocking(&self) -> Result<AuditorClientBlocking, Error> {
        Ok(AuditorClientBlocking {
            // Must clone here because `build` moves the builder, but python
            // does not allow that. Doesn't matter, Python is slow anyways.
            inner: self.inner.clone().build_blocking()?,
        })
    }
}

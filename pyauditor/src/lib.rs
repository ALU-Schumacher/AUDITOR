use anyhow::Error;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn pyauditor(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AuditorClient>()?;
    m.add_class::<AuditorClientBuilder>()?;
    Ok(())
}

#[pyclass]
pub struct AuditorClient {
    inner: auditor::client::AuditorClient,
}

#[pyclass]
pub struct AuditorClientBuilder {
    inner: auditor::client::AuditorClientBuilder,
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

// #[pymethods]
// impl AuditorClient {
//     pub fn health_check(self_: PyRef<'_, Self>, py: Python) -> PyResult<&PyAny> {
//         let locals = pyo3_asyncio::tokio::get_current_locals(py)?;
//
//         pyo3_asyncio::tokio::future_into_py_with_locals(
//             py,
//             locals.clone(),
//             // Store the current locals in task-local data
//             pyo3_asyncio::tokio::scope(locals.clone(), async move {
//                 // let py_sleep = Python::with_gil(|py| {
//                 //     pyo3_asyncio::into_future_with_locals(
//                 //         // Now we can get the current locals through task-local data
//                 //         &pyo3_asyncio::tokio::get_current_locals(py)?,
//                 //         py.import("asyncio")?.call_method1("sleep", (1,))?,
//                 //     )
//                 // })?;
//
//                 // py_sleep.await?;
//
//                 Ok(Python::with_gil(|py| py.None()))
//             }),
//         )
//         // pyo3_asyncio::tokio::future_into_py(py, async {
//         //     self_.inner.health_check();
//         //     Ok(Python::with_gil(|py| py.None()))
//         // })
//     }
// }

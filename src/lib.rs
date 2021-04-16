use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// The main Python module.
#[pymodule]
fn ahocorasick_rs(py: Python, m: &PyModule) -> PyResult<()> {
    Ok(())
}

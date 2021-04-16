use aho_corasick::AhoCorasick;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

/// A Python wrapper for AhoCorasick.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
}

/// Methods for PyAhoCorasick.
#[pymethods]
impl PyAhoCorasick {
    /// __new__() implementation.
    #[new]
    fn new(patterns: Vec<&str>) -> Self {
        Self {
            ac_impl: AhoCorasick::new(patterns),
        }
    }

    /// Return matches as indexes into original list of strings.
    fn find_matches_as_indexes(&self, haystack: &str) -> Vec<usize> {
        self.ac_impl
            .find_iter(haystack)
            .map(|m| m.pattern())
            .collect()
    }
}

/// The main Python module.
#[pymodule]
fn ahocorasick_rs(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAhoCorasick>()?;
    Ok(())
}

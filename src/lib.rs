use aho_corasick::AhoCorasick;
use pyo3::{prelude::*, types::PyUnicode};

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

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack).
    fn find_matches_as_indexes(&self, haystack: &str) -> Vec<(usize, usize, usize)> {
        self.ac_impl
            .find_iter(haystack)
            .map(|m| (m.pattern(), m.start(), m.end()))
            .collect()
    }

    /// Return matches as tuple of (pattern, start_index_in_haystack).
    fn find_matches_as_strings(self_: PyRef<Self>, haystack: &str) -> Vec<(Py<PyAny>, usize)> {
        self_
            .ac_impl
            .find_iter(&haystack)
            .map(|m| {
                (
                    haystack[m.start()..m.end()].to_object(self_.py()),
                    m.start(),
                )
            })
            .collect()
    }
}

/// The main Python module.
#[pymodule]
fn ahocorasick_rs(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAhoCorasick>()?;
    Ok(())
}

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use pyo3::{exceptions::PyValueError, prelude::*};

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
    #[args(matchkind = "\"MATCHKIND_STANDARD\"")]
    fn new(patterns: Vec<&str>, matchkind: &str) -> PyResult<Self> {
        let matchkind = match matchkind {
            "MATCHKIND_STANDARD" => MatchKind::Standard,
            "MATCHKIND_LEFTMOST_FIRST" => MatchKind::LeftmostFirst,
            "MATCHKIND_LEFTMOST_LONGEST" => MatchKind::LeftmostLongest,
            _ => {
                return Err(PyValueError::new_err(
                    "matchkind must be one of the ahocorasick_rs.MATCHKIND_* constants.",
                ));
            }
        };
        Ok(Self {
            ac_impl: AhoCorasickBuilder::new()
                .match_kind(matchkind)
                .build(patterns),
        })
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
fn ahocorasick_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAhoCorasick>()?;
    // PyO3 doesn't support auto-wrapping Enums, so we just do it manually.
    m.add("MATCHKIND_STANDARD", "MATCHKIND_STANDARD")?;
    m.add("MATCHKIND_LEFTMOST_FIRST", "MATCHKIND_LEFTMOST_FIRST")?;
    m.add("MATCHKIND_LEFTMOST_LONGEST", "MATCHKIND_LEFTMOST_LONGEST")?;
    Ok(())
}

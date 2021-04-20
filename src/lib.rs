use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Match, MatchKind};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyUnicode};

/// A Python wrapper for AhoCorasick.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Vec<Py<PyUnicode>>,
}

impl<'a> PyAhoCorasick {
    fn get_iter(
        &'a self,
        haystack: &'a str,
        overlapping: bool,
    ) -> PyResult<Box<dyn Iterator<Item = Match> + 'a>> {
        if overlapping && !self.ac_impl.supports_overlapping() {
            return Err(PyValueError::new_err("This automaton doesn't support overlapping results; perhaps you didn't use the defalt matchkind (MATCHKIND_STANDARD)?"));
        }
        Ok(if overlapping {
            Box::new(self.ac_impl.find_overlapping_iter(haystack))
        } else {
            Box::new(self.ac_impl.find_iter(haystack))
        })
    }
}

/// Methods for PyAhoCorasick.
#[pymethods]
impl PyAhoCorasick {
    /// __new__() implementation.
    #[new]
    #[args(matchkind = "\"MATCHKIND_STANDARD\"")]
    fn new(py: Python, patterns: Vec<Py<PyUnicode>>, matchkind: &str) -> PyResult<Self> {
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
        let mut rust_patterns: Vec<String> = vec![];
        for s in patterns.iter() {
            rust_patterns.push(s.as_ref(py).extract()?);
        }
        Ok(Self {
            ac_impl: AhoCorasickBuilder::new()
                .dfa(true) // DFA results in faster matches
                .match_kind(matchkind)
                .build(rust_patterns),
            patterns,
        })
    }

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack).
    #[args(overlapping = "false")]
    fn find_matches_as_indexes(
        &self,
        haystack: String,
        overlapping: bool,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        Ok(self
            .get_iter(&haystack, overlapping)?
            .map(|m| (m.pattern(), m.start(), m.end()))
            .collect())
    }

    /// Return matches as tuple of (pattern, start_index_in_haystack).
    #[args(overlapping = "false")]
    fn find_matches_as_strings(
        self_: PyRef<Self>,
        haystack: String,
        overlapping: bool,
    ) -> PyResult<Vec<(Py<PyUnicode>, usize)>> {
        Ok(self_
            .get_iter(&haystack, overlapping)?
            .map(|m| (self_.patterns[m.pattern()].clone(), m.start()))
            .collect())
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

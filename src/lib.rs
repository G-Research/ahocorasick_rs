use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Match, MatchKind};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyUnicode};

/// A Python wrapper for AhoCorasick.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Vec<Py<PyUnicode>>,
}

impl PyAhoCorasick {
    fn check_overlapping(&self, overlapping: bool) -> PyResult<()> {
        if overlapping && !self.ac_impl.supports_overlapping() {
            return Err(PyValueError::new_err("This automaton doesn't support overlapping results; perhaps you didn't use the defalt matchkind (MATCHKIND_STANDARD)?"));
        }
        Ok(())
    }

    /// Return matches for a given haystack.
    fn get_matches(&self, py: Python<'_>, haystack: &str, overlapping: bool) -> Vec<Match> {
        let ac_impl = &self.ac_impl;
        py.allow_threads(|| {
            if overlapping {
                ac_impl.find_overlapping_iter(haystack).collect()
            } else {
                ac_impl.find_iter(haystack).collect()
            }
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
            ac_impl: py.allow_threads(|| {
                AhoCorasickBuilder::new()
                    .dfa(true) // DFA results in faster matches
                    .match_kind(matchkind)
                    .build(rust_patterns)
            }),
            patterns,
        })
    }

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack).
    #[args(overlapping = "false")]
    fn find_matches_as_indexes(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        self_.check_overlapping(overlapping)?;
        // Map UTF-8 byte index to Unicode code point index; the latter is what
        // Python users expect.
        let mut byte_to_code_point = vec![usize::MAX; haystack.len() + 1];
        let mut max_codepoint = 0;
        for (codepoint_off, (byte_off, _)) in haystack.char_indices().enumerate() {
            byte_to_code_point[byte_off] = codepoint_off;
            max_codepoint = codepoint_off;
        }
        // End index is exclusive (e.g. 0:3 is first 3 characters), so handle
        // the case where pattern is at end of string.
        if haystack.len() > 0 {
            byte_to_code_point[haystack.len()] = max_codepoint + 1;
        }
        let py = self_.py();
        let matches = self_.get_matches(py, haystack, overlapping);
        Ok(matches
            .into_iter()
            .map(|m| {
                (
                    m.pattern(),
                    byte_to_code_point[m.start()],
                    byte_to_code_point[m.end()],
                )
            })
            .collect())
    }

    /// Return matches as list of patterns.
    #[args(overlapping = "false")]
    fn find_matches_as_strings(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<Py<PyUnicode>>> {
        self_.check_overlapping(overlapping)?;
        let py = self_.py();
        let matches = self_.get_matches(py, haystack, overlapping);
        Ok(matches
            .into_iter()
            .map(|m| self_.patterns[m.pattern()].clone_ref(py))
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

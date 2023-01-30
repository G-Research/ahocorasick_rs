use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Match, MatchKind};
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyList, PyUnicode},
};

/// Search for multiple pattern strings against a single haystack string.
///
/// Takes three arguments:
///
/// * ``patterns``: A list of strings, the patterns to match against.
/// * ``matchkind``: Defaults to ``"MATCHKING_STANDARD"``.
/// * ``store_patterns``: If ``True``, keep a reference to the patterns, which
///   will speed up ``find_matches_as_strings()`` but will use more memory. If
///   ``False``, patterns will not be stored. By default uses a heuristic where
///   a short list of small strings (up to 4KB) results in ``True``, and
///   anything else results in ``False``.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Option<Vec<Py<PyUnicode>>>,
}

impl PyAhoCorasick {
    /// Raise a Python ValueError if the request overlapping option is not
    /// supported.
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

    /// Create mapping from byte index to Unicode code point (character) index
    /// in the haystack.
    fn get_byte_to_code_point(&self, haystack: &str) -> Vec<usize> {
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
        if !haystack.is_empty() {
            byte_to_code_point[haystack.len()] = max_codepoint + 1;
        }
        byte_to_code_point
    }
}

/// Methods for PyAhoCorasick.
#[pymethods]
impl PyAhoCorasick {
    /// __new__() implementation.
    #[new]
    #[pyo3(signature = (patterns, matchkind = "MATCHKIND_STANDARD", store_patterns = None))]
    fn new(
        py: Python,
        patterns: Vec<Py<PyUnicode>>,
        matchkind: &str,
        store_patterns: Option<bool>,
    ) -> PyResult<Self> {
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
        // If store_patterns is None (the default), use a heuristic to decide
        // whether to store patterns.
        let store_patterns = store_patterns.unwrap_or_else(|| {
            patterns
                .iter()
                // It's very unlikely we won't be able to get the length...
                .map(|s| s.as_ref(py).len().unwrap())
                .sum::<usize>()
                <= 4096
        });
        Ok(Self {
            ac_impl: AhoCorasickBuilder::new()
                .dfa(true) // DFA results in faster matches
                .match_kind(matchkind)
                .build(patterns.chunks(10 * 1024).flat_map(|chunk| {
                    let result = chunk
                        .iter()
                        .filter_map(|s| s.as_ref(py).extract::<String>().ok());
                    // Release the GIL in case some other thread wants to do work:
                    py.allow_threads(|| ());
                    result
                })),
            patterns: store_patterns.then_some(patterns),
        })
    }

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack). If ``overlapping`` is
    /// ``False`` (the default), don't include overlapping results.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_indexes(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        self_.check_overlapping(overlapping)?;
        let byte_to_code_point = self_.get_byte_to_code_point(haystack);
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

    /// Return matches as list of patterns (i.e. strings). If ``overlapping`` is
    /// ``False`` (the default), don't include overlapping results.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_strings(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Py<PyList>> {
        self_.check_overlapping(overlapping)?;
        let py = self_.py();
        let matches = self_.get_matches(py, haystack, overlapping).into_iter();
        let result = if let Some(ref patterns) = self_.patterns {
            PyList::new(py, matches.map(|m| patterns[m.pattern()].clone_ref(py)))
        } else {
            PyList::new(
                py,
                matches.map(|m| PyUnicode::new(py, &haystack[m.start()..m.end()])),
            )
        };
        Ok(result.into())
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

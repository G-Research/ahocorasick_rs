use std::cell::Cell;

use aho_corasick::{
    AhoCorasick, AhoCorasickBuilder, AhoCorasickKind, Match, MatchError, MatchKind,
};
use itertools::Itertools;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyList, PyUnicode},
};

/// Search for multiple pattern strings against a single haystack string.
///
/// Takes four arguments:
///
/// * ``patterns``: A list of strings, the patterns to match against. Empty
///   patterns are not supported and will result in a ``ValueError`` exception
///   being raised.
/// * ``matchkind``: Defaults to ``"MATCHKING_STANDARD"``.
/// * ``store_patterns``: If ``True``, keep a reference to the patterns, which
///   will speed up ``find_matches_as_strings()`` but will use more memory. If
///   ``False``, patterns will not be stored. By default uses a heuristic where
///   a short list of small strings (up to 4KB) results in ``True``, and
///   anything else results in ``False``.
/// * ``implementation``: The underlying type of automaton to use for Aho-Corasick.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Option<Vec<Py<PyUnicode>>>,
}

/// Convert a MatchError to something meaningful to Python users
#[cold]
fn match_error_to_pyerror(e: MatchError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

impl PyAhoCorasick {
    /// Return matches for a given haystack.
    fn get_matches(
        &self,
        py: Python<'_>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<Match>> {
        let ac_impl = &self.ac_impl;
        py.allow_threads(|| {
            if overlapping {
                ac_impl
                    .try_find_overlapping_iter(haystack)
                    .map_err(match_error_to_pyerror)
                    .map(|it| it.collect())
            } else {
                Ok(ac_impl.find_iter(haystack).collect())
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

/// Python equivalent of MatchKind.
#[derive(Clone, Copy, Debug)]
#[pyclass(name = "MatchKind")]
enum PyMatchKind {
    Standard,
    LeftmostFirst,
    LeftmostLongest,
}

impl From<PyMatchKind> for MatchKind {
    fn from(value: PyMatchKind) -> Self {
        match value {
            PyMatchKind::Standard => Self::Standard,
            PyMatchKind::LeftmostFirst => Self::LeftmostFirst,
            PyMatchKind::LeftmostLongest => Self::LeftmostLongest,
        }
    }
}

/// Python equivalent of AhoCorasickKind.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::upper_case_acronyms)]
#[pyclass]
enum Implementation {
    NoncontiguousNFA,
    ContiguousNFA,
    DFA,
}

impl From<Implementation> for AhoCorasickKind {
    fn from(value: Implementation) -> Self {
        match value {
            Implementation::NoncontiguousNFA => Self::NoncontiguousNFA,
            Implementation::ContiguousNFA => Self::ContiguousNFA,
            Implementation::DFA => Self::DFA,
        }
    }
}

/// Methods for PyAhoCorasick.
#[pymethods]
impl PyAhoCorasick {
    /// __new__() implementation.
    #[new]
    #[pyo3(signature = (patterns, matchkind = PyMatchKind::Standard, store_patterns = None, implementation = None))]
    fn new(
        py: Python,
        patterns: &PyAny,
        matchkind: PyMatchKind,
        store_patterns: Option<bool>,
        implementation: Option<Implementation>,
    ) -> PyResult<Self> {
        // If set, this means we had an error while parsing the strings from the patterns iterable.
        let patterns_error: Cell<Option<PyErr>> = Cell::new(None);

        // Convert the `patterns` iterable into an Iterator over &PyUnicode:
        let mut patterns_iter = patterns.iter()?.map_while(|pat| {
            pat.and_then(|i| i.downcast::<PyUnicode>().map_err(PyErr::from))
                .map_or_else(
                    |e| {
                        patterns_error.set(Some(e));
                        None
                    },
                    Some,
                )
        });

        // If store_patterns is None (the default), use a heuristic to decide
        // whether to store patterns.
        let mut patterns: Vec<Py<PyUnicode>> = vec![];
        let store_patterns = store_patterns
            .unwrap_or_else(|| {
                let mut total = 0;
                let mut store_patterns = true;
                for s in patterns_iter.by_ref() {
                    // Highly unlikely that strings will fail to return length, so just expect().
                    total += s.len().expect("String doesn't have length?");
                    patterns.push(s.into());
                    if total > 4096 {
                        store_patterns = false;
                        break;
                    }
                }
                store_patterns
            });

        if store_patterns {
            for s in patterns_iter.by_ref() {
                patterns.push(s.into());
            }
        }

        let ac_impl = AhoCorasickBuilder::new()
            .kind(implementation.map(|i| i.into()))
            .match_kind(matchkind.into())
            .build(
                patterns
                    .iter()
                    .map(|i| i.as_ref(py))
                    .chain(patterns_iter)
                    .chunks(10 * 1024)
                    .into_iter()
                    .flat_map(|chunk| {
                        // Release the GIL in case some other thread wants to do work:
                        py.allow_threads(|| ());

                        chunk.map(|s| s.extract::<String>().ok())
                    })
                    .map_while(|s| {
                        s.and_then(|s| {
                            if s.is_empty() {
                                patterns_error.set(Some(PyValueError::new_err(
                                    "You passed in an empty string as a pattern",
                                )));
                                None
                            } else {
                                Some(s)
                            }
                        })
                    }),
            ) // TODO make sure this error is meaningful to Python users
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        if let Some(err) = patterns_error.take() {
            return Err(err);
        }

        let patterns = if store_patterns { Some(patterns) } else { None };

        Ok(Self { ac_impl, patterns })
    }

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack). If ``overlapping`` is
    /// ``False`` (the default), don't include overlapping results.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_indexes(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<(u64, usize, usize)>> {
        let byte_to_code_point = self_.get_byte_to_code_point(haystack);
        let py = self_.py();
        let matches = self_.get_matches(py, haystack, overlapping)?;
        Ok(matches
            .into_iter()
            .map(|m| {
                (
                    m.pattern().as_u64(),
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
        let py = self_.py();
        let matches = self_.get_matches(py, haystack, overlapping)?.into_iter();
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
    m.add_class::<PyMatchKind>()?;
    m.add_class::<Implementation>()?;
    m.add_class::<PyAhoCorasick>()?;
    Ok(())
}

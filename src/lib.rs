use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Match, MatchKind};
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyList, PyUnicode},
};
use rayon::prelude::*;

/// A Python wrapper for AhoCorasick.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Vec<Py<PyUnicode>>,
}

impl<'a> PyAhoCorasick {
    fn check_overlapping(&self, overlapping: bool) -> PyResult<()> {
        if overlapping && !self.ac_impl.supports_overlapping() {
            return Err(PyValueError::new_err("This automaton doesn't support overlapping results; perhaps you didn't use the defalt matchkind (MATCHKIND_STANDARD)?"));
        }
        Ok(())
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
        haystack: String,
        overlapping: bool,
    ) -> PyResult<Vec<(usize, usize, usize)>> {
        self_.check_overlapping(overlapping)?;
        let ac_impl = &self_.ac_impl;
        self_.py().allow_threads(|| {
            if overlapping {
                Ok(ac_impl
                    .find_overlapping_iter(&haystack)
                    .map(|m| (m.pattern(), m.start(), m.end()))
                    .collect())
            } else {
                Ok(ac_impl
                    .find_iter(&haystack)
                    .map(|m| (m.pattern(), m.start(), m.end()))
                    .collect())
            }
        })
    }

    /// Return matches as list of patterns.
    #[args(overlapping = "false")]
    fn find_matches_as_strings(
        self_: PyRef<Self>,
        haystack: &str,
        overlapping: bool,
    ) -> PyResult<Vec<Py<PyUnicode>>> {
        let py = self_.py();
        self_.check_overlapping(overlapping)?;
        let ac_impl = &self_.ac_impl;
        if overlapping {
            Ok(py
                .allow_threads(|| ac_impl.find_overlapping_iter(haystack))
                .map(|m| self_.patterns[m.pattern()].clone_ref(py))
                .collect())
        } else {
            Ok(py
                .allow_threads(|| ac_impl.find_iter(haystack))
                .map(|m| self_.patterns[m.pattern()].clone_ref(py))
                .collect())
        }
    }

    fn parallel_map_to_strings(self_: PyRef<Self>, haystacks: Vec<&str>) -> PyResult<PyObject> {
        let py = self_.py();
        let ac_impl = &self_.ac_impl;

        let matches: Vec<Vec<usize>> = py.allow_threads(|| {
            haystacks
                .par_iter()
                .map(|haystack| {
                    ac_impl
                        .find_iter(haystack)
                        .map(|pattern_match| pattern_match.pattern())
                        .collect()
                })
                .collect()
        });
        Ok(PyList::new(
            py,
            matches.into_iter().map(|vm| {
                PyList::new(
                    py,
                    vm.into_iter()
                        .map(|pattern_index| self_.patterns[pattern_index].clone_ref(py)),
                )
            }),
        )
        .into())
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

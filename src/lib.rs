use std::cell::Cell;

use aho_corasick::{
    AhoCorasick, AhoCorasickBuilder, AhoCorasickKind, Match, MatchError, MatchKind,
};
use itertools::Itertools;
use pyo3::{
    buffer::{PyBuffer, ReadOnlyCell},
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyList, PyString},
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
    patterns: Option<Vec<Py<PyString>>>,
}

/// Convert a MatchError to something meaningful to Python users
#[cold]
fn match_error_to_pyerror(e: MatchError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Return matches for a given haystack.
fn get_matches<'a>(
    ac_impl: &'a AhoCorasick,
    haystack: &'a [u8],
    overlapping: bool,
) -> PyResult<impl Iterator<Item = Match> + 'a> {
    let mut overlapping_it = None;
    let mut non_overlapping_it = None;

    if overlapping {
        overlapping_it = Some(
            ac_impl
                .try_find_overlapping_iter(haystack)
                .map_err(match_error_to_pyerror)?,
        );
    } else {
        non_overlapping_it = Some(
            ac_impl
                .try_find_iter(haystack)
                .map_err(match_error_to_pyerror)?,
        );
    }

    Ok(overlapping_it
        .into_iter()
        .flatten()
        .chain(non_overlapping_it.into_iter().flatten()))
}

impl PyAhoCorasick {
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
#[derive(Clone, Copy, Debug, PartialEq)]
#[pyclass(eq, name = "MatchKind")]
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
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
#[pyclass(eq)]
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
        patterns: Bound<'_, PyAny>,
        matchkind: PyMatchKind,
        store_patterns: Option<bool>,
        implementation: Option<Implementation>,
    ) -> PyResult<Self> {
        // If set, this means we had an error while parsing the strings from the patterns iterable.
        let patterns_error: Cell<Option<PyErr>> = Cell::new(None);

        // Convert the `patterns` iterable into an Iterator over Py<PyString>:
        let mut patterns_iter = patterns.try_iter()?.map_while(|pat| {
            pat.and_then(|i| {
                i.downcast_into::<PyString>()
                    .map_err(PyErr::from)
                    .map(|i| i.unbind())
            })
            .map_or_else(
                |e| {
                    patterns_error.set(Some(e));
                    None
                },
                Some::<Py<PyString>>,
            )
        });

        // If store_patterns is None (the default), use a heuristic to decide
        // whether to store patterns.
        let mut patterns: Vec<Py<PyString>> = vec![];
        let store_patterns = store_patterns.unwrap_or_else(|| {
            let mut total = 0;
            let mut store_patterns = true;
            for s in patterns_iter.by_ref() {
                // Highly unlikely that strings will fail to return length, so just expect().
                total += s.bind(py).len().expect("String doesn't have length?");
                patterns.push(s);
                if total > 4096 {
                    store_patterns = false;
                    break;
                }
            }
            store_patterns
        });

        if store_patterns {
            for s in patterns_iter.by_ref() {
                patterns.push(s);
            }
        }

        let ac_impl = AhoCorasickBuilder::new()
            .kind(implementation.map(|i| i.into()))
            .match_kind(matchkind.into())
            .build(
                patterns
                    .clone()
                    .into_iter()
                    .chain(patterns_iter)
                    .chunks(10 * 1024)
                    .into_iter()
                    .flat_map(|chunk| {
                        // Release the GIL in case some other thread wants to do work:
                        py.detach(|| ());

                        chunk.map(|s| s.extract::<String>(py).ok())
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
        let matches = get_matches(&self_.ac_impl, haystack.as_bytes(), overlapping)?;
        py.detach(|| {
            Ok(matches
                .map(|m| {
                    (
                        m.pattern().as_u64(),
                        byte_to_code_point[m.start()],
                        byte_to_code_point[m.end()],
                    )
                })
                .collect())
        })
    }

    /// Return matches as list of patterns (i.e. strings). If ``overlapping`` is
    /// ``False`` (the default), don't include overlapping results.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_strings<'py>(
        self_: PyRef<'py, Self>,
        haystack: &'py str,
        overlapping: bool,
    ) -> PyResult<Bound<'py, PyList>> {
        let py = self_.py();
        let matches = get_matches(&self_.ac_impl, haystack.as_bytes(), overlapping)?;
        let matches = py.detach(|| matches.collect::<Vec<_>>().into_iter());

        match self_.patterns {
            Some(ref patterns) => {
                PyList::new(py, matches.map(|m| patterns[m.pattern()].clone_ref(py)))
            }
            _ => PyList::new(
                py,
                matches.map(|m| PyString::new(py, &haystack[m.start()..m.end()])),
            ),
        }
    }
}

/// A wrapper around PyBuffer that can be passed directly to AhoCorasickBuilder.
struct PyBufferBytes<'py> {
    py: Python<'py>,
    buffer: PyBuffer<u8>,
}

impl<'py> TryFrom<Bound<'py, PyAny>> for PyBufferBytes<'py> {
    type Error = PyErr;

    // Get a PyBufferBytes from a Python object
    fn try_from(obj: Bound<'py, PyAny>) -> PyResult<Self> {
        let buffer = PyBuffer::<u8>::get(&obj)?;

        if buffer.dimensions() > 1 {
            return Err(PyTypeError::new_err(
                "Only one-dimensional sequences are supported",
            ));
        }

        // Make sure we can get a slice from the buffer
        let py = obj.py();
        let _ = buffer
            .as_slice(py)
            .ok_or_else(|| PyTypeError::new_err("Must be a contiguous sequence of bytes"))?;

        Ok(PyBufferBytes { py, buffer })
    }
}

impl<'a> AsRef<[u8]> for PyBufferBytes<'a> {
    fn as_ref(&self) -> &[u8] {
        // This already succeeded when we created PyBufferBytes, so just expect()
        let slice = self
            .buffer
            .as_slice(self.py)
            .expect("Failed to get a slice from a valid buffer?");

        const _: () = assert!(
            std::mem::size_of::<ReadOnlyCell<u8>>() == std::mem::size_of::<u8>(),
            "ReadOnlyCell<u8> has a different size than u8"
        );

        // Safety: the slice is &[ReadOnlyCell<u8>] which has the same memory
        // representation as &[u8] due to it being a #[repr(transparent)] newtype
        // around the standard library UnsafeCell<u8>, which the documentation
        // claims has the same representation as the underlying type.
        //
        // However, holding this reference while Python code is executing might
        // result in the buffer getting mutated from under us, which is a violation
        // of the &[u8] invariants (and having the .readonly() flag set on the
        // PyBuffer unfortunately doesn't actually guarantee immutability).
        //
        // Because &[u8] is `Ungil`, we can't prevent a release of the GIL while
        // this reference is being held (though even if it wasn't `Ungil`, we
        // wouldn't be able to prevent calling back into Python while holding
        // this reference, which might also result in a mutation).
        //
        // In addition, in a free-threaded world there is no GIL at all to
        // prevent mutation.
        //
        // Following the lead of `pyo3-numpy`, we deal with this by documenting
        // to the user that the buffer cannot be mutated while it is passed to
        // our API. See also https://github.com/PyO3/pyo3/issues/2824
        unsafe { std::mem::transmute(slice) }
    }
}

/// Search for multiple pattern bytes against a single bytes haystack. In
/// addition to ``bytes``, you can use other objects that support the Python
/// buffer API, like ``memoryview`` and ``bytearray``.
///
/// Takes three arguments:
///
/// * ``patterns``: A list of bytes, the patterns to match against. Empty
///   patterns are not supported and will result in a ``ValueError`` exception
///   being raised. No references are kept to the patterns once construction is
///   finished.
/// * ``matchkind``: Defaults to ``"MATCHKING_STANDARD"``.
/// * ``implementation``: The underlying type of automaton to use for Aho-Corasick.
///
/// IMPORTANT: If you are passing in patterns that are mutable buffers, you MUST
/// NOT mutate then in another thread while constructing this object. Doing so
/// will result in undefined behavior. Once the ``BytesAhoCorasick`` object is
/// constructed, however, they can be mutated since no references will be kept
/// to them.
#[pyclass(name = "BytesAhoCorasick")]
struct PyBytesAhoCorasick {
    ac_impl: AhoCorasick,
}

/// Methods for PyBytesAhoCorasick.
#[pymethods]
impl PyBytesAhoCorasick {
    /// __new__() implementation.
    #[new]
    #[pyo3(signature = (patterns, matchkind = PyMatchKind::Standard, implementation = None))]
    fn new(
        _py: Python,
        patterns: Bound<'_, PyAny>,
        matchkind: PyMatchKind,
        implementation: Option<Implementation>,
    ) -> PyResult<Self> {
        // If set, this means we had an error while parsing byte buffers from `patterns`
        let patterns_error: Cell<Option<PyErr>> = Cell::new(None);

        // Convert the `patterns` iterable into an Iterator over PyBufferBytes
        let patterns_iter =
            patterns
                .try_iter()?
                .map_while(|pat| match pat.and_then(PyBufferBytes::try_from) {
                    Ok(pat) => {
                        if pat.as_ref().is_empty() {
                            patterns_error.set(Some(PyValueError::new_err(
                                "You passed in an empty pattern",
                            )));
                            None
                        } else {
                            Some(pat)
                        }
                    }
                    Err(e) => {
                        patterns_error.set(Some(e));
                        None
                    }
                });

        let ac_impl = AhoCorasickBuilder::new()
            .kind(implementation.map(|i| i.into()))
            .match_kind(matchkind.into())
            .build(patterns_iter)
            // TODO make sure this error is meaningful to Python users
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        if let Some(err) = patterns_error.take() {
            return Err(err);
        }

        Ok(Self { ac_impl })
    }

    /// Return matches as tuple of (index_into_patterns,
    /// start_index_in_haystack, end_index_in_haystack). If ``overlapping`` is
    /// ``False`` (the default), don't include overlapping results.
    ///
    /// IMPORTANT: If you are passing in a mutable buffer, you MUST NOT mutate
    /// it in another thread while this API is running. Doing so will result in
    /// undefined behavior.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_indexes(
        self_: PyRef<Self>,
        haystack: Bound<'_, PyAny>,
        overlapping: bool,
    ) -> PyResult<Vec<(u64, usize, usize)>> {
        let py = haystack.py();
        let haystack_buffer = PyBufferBytes::try_from(haystack)?;
        let matches = get_matches(&self_.ac_impl, haystack_buffer.as_ref(), overlapping)?
            .map(|m| (m.pattern().as_u64(), m.start(), m.end()));

        py.detach(|| Ok(matches.collect()))
    }
}

/// The main Python module.
#[pymodule(gil_used = false)]
fn ahocorasick_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMatchKind>()?;
    m.add_class::<Implementation>()?;
    m.add_class::<PyAhoCorasick>()?;
    m.add_class::<PyBytesAhoCorasick>()?;
    Ok(())
}

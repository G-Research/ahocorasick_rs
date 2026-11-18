use std::{cell::Cell, num::NonZeroUsize};

use aho_corasick::{
    AhoCorasick, AhoCorasickBuilder, AhoCorasickKind, Input, Match, MatchError, MatchKind,
    automaton::OverlappingState,
};
use itertools::Itertools;
use pyo3::{
    buffer::{PyBuffer, ReadOnlyCell},
    exceptions::{PyTypeError, PyValueError},
    gc::{PyTraverseError, PyVisit},
    prelude::*,
    pybacked::PyBackedStr,
    types::{PyBytes, PyList, PyMemoryView, PyString},
};

/// Search for multiple pattern strings against a single haystack string.
///
/// The constructor accepts five arguments.
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
/// * ``iter_batch_size``: Maximum matches buffered per iterator refill (default
///   128). Use 1 to search one match at a time.
#[pyclass(name = "AhoCorasick")]
struct PyAhoCorasick {
    ac_impl: AhoCorasick,
    patterns: Option<Vec<Py<PyString>>>,
    iter_batch_size: NonZeroUsize,
}

/// Convert a MatchError to something meaningful to Python users
#[cold]
fn match_error_to_pyerror(e: MatchError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn validate_iter_batch_size(iter_batch_size: usize) -> PyResult<NonZeroUsize> {
    NonZeroUsize::new(iter_batch_size)
        .ok_or_else(|| PyValueError::new_err("iter_batch_size must be positive"))
}

/// Resumable match traversal. The caller supplies the same haystack for every scan.
struct MatchCursor {
    overlapping: Option<OverlappingState>,
    end_byte: usize,
}

impl MatchCursor {
    fn new(ac_impl: &AhoCorasick, overlapping: bool) -> Result<Self, MatchError> {
        // Both Python matchers use the default unanchored start kind. Only
        // overlapping searches can be incompatible with the chosen match kind.
        if overlapping {
            // Validate the search mode without reading the haystack.
            ac_impl.try_find_overlapping_iter(b"")?;
        }
        Ok(Self {
            overlapping: overlapping.then(OverlappingState::start),
            end_byte: 0,
        })
    }

    /// Get a match at the last endpoint without accessing the haystack.
    #[inline]
    fn next_pending(&mut self, ac_impl: &AhoCorasick) -> Option<Match> {
        ac_impl.next_pending_overlapping_match(self.overlapping.as_mut()?)
    }

    #[inline]
    fn scan(
        &mut self,
        ac_impl: &AhoCorasick,
        haystack: &[u8],
    ) -> Result<Option<Match>, MatchError> {
        let found = if let Some(state) = &mut self.overlapping {
            ac_impl.try_find_overlapping(haystack, state)?;
            state.get_match()
        } else {
            // The Python constructors reject empty patterns, so each match advances.
            ac_impl.try_find(Input::new(haystack).span(self.end_byte..haystack.len()))?
        };
        if let Some(mat) = found {
            self.end_byte = mat.end();
        }
        Ok(found)
    }

    #[inline]
    fn next_match(
        &mut self,
        ac_impl: &AhoCorasick,
        haystack: &[u8],
    ) -> Result<Option<Match>, MatchError> {
        match self.next_pending(ac_impl) {
            Some(mat) => Ok(Some(mat)),
            None => self.scan(ac_impl, haystack),
        }
    }

    #[inline]
    fn collect_batch(
        &mut self,
        ac_impl: &AhoCorasick,
        haystack: &[u8],
        batch_size: usize,
        mut to_pair: impl FnMut(usize, Match) -> (usize, u64),
    ) -> Result<Vec<(usize, u64)>, MatchError> {
        let mut batch = Vec::new();
        for _ in 0..batch_size {
            let previous_end_byte = self.end_byte;
            let Some(mat) = self.next_match(ac_impl, haystack)? else {
                break;
            };
            batch.push(to_pair(previous_end_byte, mat));
        }
        Ok(batch)
    }
}

/// Return matches for a given haystack. Eager callers consume this while detached.
fn get_matches<'a>(
    ac_impl: &'a AhoCorasick,
    haystack: &'a [u8],
    overlapping: bool,
) -> PyResult<impl Iterator<Item = Match> + 'a> {
    let mut cursor = MatchCursor::new(ac_impl, overlapping).map_err(match_error_to_pyerror)?;
    Ok(std::iter::from_fn(move || {
        cursor
            .next_match(ac_impl, haystack)
            .expect("search mode validated at cursor construction")
    }))
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
#[derive(Clone, Debug, PartialEq)]
#[pyclass(eq, name = "MatchKind", from_py_object)]
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
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
#[pyclass(eq, from_py_object)]
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
    #[pyo3(signature = (patterns, matchkind = PyMatchKind::Standard, store_patterns = None, implementation = None, *, iter_batch_size = 128))]
    fn new(
        py: Python,
        patterns: Bound<'_, PyAny>,
        matchkind: PyMatchKind,
        store_patterns: Option<bool>,
        implementation: Option<Implementation>,
        iter_batch_size: usize,
    ) -> PyResult<Self> {
        let iter_batch_size = validate_iter_batch_size(iter_batch_size)?;
        // If set, this means we had an error while parsing the strings from the patterns iterable.
        let patterns_error: Cell<Option<PyErr>> = Cell::new(None);

        // Convert the `patterns` iterable into an Iterator over Py<PyString>:
        let mut patterns_iter = patterns.try_iter()?.map_while(|pat| {
            pat.and_then(|i| {
                i.cast_into::<PyString>()
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

                        chunk.map(|s| s.extract::<PyBackedStr>(py).ok())
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

        Ok(Self {
            ac_impl,
            patterns,
            iter_batch_size,
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

    /// Return a lazy iterator of ``(end_index_in_haystack,
    /// index_into_patterns)`` tuples, in the same order as
    /// ``find_matches_as_indexes()``.
    ///
    /// After the first match, the iterator may search ahead according to
    /// ``iter_batch_size`` on the matcher.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_indexes_iter(
        self_: Py<Self>,
        py: Python<'_>,
        haystack: Bound<'_, PyString>,
        overlapping: bool,
    ) -> PyResult<Py<PyMatchIterator>> {
        let haystack = PyBackedStr::try_from(haystack)?;
        let (ac_impl, iter_batch_size) = {
            let matcher = self_.bind(py).borrow();
            (matcher.ac_impl.clone(), matcher.iter_batch_size)
        };
        let iterator = PyMatchIterator::new(
            ac_impl,
            MatchHaystack::String {
                haystack,
                end_code_point: 0,
            },
            overlapping,
            iter_batch_size,
        )?;
        Py::new(py, iterator)
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

/// Get a validated one-dimensional contiguous buffer from a Python object.
fn get_byte_buffer(obj: &Bound<'_, PyAny>) -> PyResult<PyBuffer<u8>> {
    let buffer = PyBuffer::<u8>::get(obj)?;

    if buffer.dimensions() > 1 {
        return Err(PyTypeError::new_err(
            "Only one-dimensional sequences are supported",
        ));
    }

    // Make sure we can get a slice from the buffer
    let _ = buffer
        .as_slice(obj.py())
        .ok_or_else(|| PyTypeError::new_err("Must be a contiguous sequence of bytes"))?;

    Ok(buffer)
}

/// Borrow the bytes of a buffer validated by `get_byte_buffer()`.
fn buffer_bytes<'a>(py: Python<'_>, buffer: &'a PyBuffer<u8>) -> &'a [u8] {
    // This already succeeded when we validated the buffer, so just expect()
    let slice = buffer
        .as_slice(py)
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

/// A wrapper around PyBuffer that can be passed directly to AhoCorasickBuilder.
struct PyBufferBytes<'py> {
    py: Python<'py>,
    buffer: PyBuffer<u8>,
}

impl<'py> TryFrom<Bound<'py, PyAny>> for PyBufferBytes<'py> {
    type Error = PyErr;

    // Get a PyBufferBytes from a Python object
    fn try_from(obj: Bound<'py, PyAny>) -> PyResult<Self> {
        Ok(PyBufferBytes {
            py: obj.py(),
            buffer: get_byte_buffer(&obj)?,
        })
    }
}

impl<'a> AsRef<[u8]> for PyBufferBytes<'a> {
    fn as_ref(&self) -> &[u8] {
        buffer_bytes(self.py, &self.buffer)
    }
}

/// Search for multiple pattern bytes against a single bytes haystack. In
/// addition to ``bytes``, you can use other objects that support the Python
/// buffer API, like ``memoryview`` and ``bytearray``.
///
/// The constructor accepts four arguments.
///
/// * ``patterns``: A list of bytes, the patterns to match against. Empty
///   patterns are not supported and will result in a ``ValueError`` exception
///   being raised. No references are kept to the patterns once construction is
///   finished.
/// * ``matchkind``: Defaults to ``"MATCHKING_STANDARD"``.
/// * ``implementation``: The underlying type of automaton to use for Aho-Corasick.
/// * ``iter_batch_size``: Maximum matches buffered per iterator refill on
///   built-in bytes (default 128). Use 1 to search one match at a time.
///
/// IMPORTANT: If you are passing in patterns that are mutable buffers, you MUST
/// NOT mutate then in another thread while constructing this object. Doing so
/// will result in undefined behavior. Once the ``BytesAhoCorasick`` object is
/// constructed, however, they can be mutated since no references will be kept
/// to them.
#[pyclass(name = "BytesAhoCorasick")]
struct PyBytesAhoCorasick {
    ac_impl: AhoCorasick,
    iter_batch_size: NonZeroUsize,
}

enum MatchHaystack {
    String {
        haystack: PyBackedStr,
        end_code_point: usize,
    },
    Bytes(Py<PyBytes>),
    /// The memoryview keeps the exporter's buffer pinned and visible to GC;
    /// the buffer is acquired from it once instead of on every `__next__()`.
    Buffer {
        view: Py<PyMemoryView>,
        buffer: PyBuffer<u8>,
    },
}

impl MatchHaystack {
    /// Collect a batch for immutable inputs. Other buffers are searched one
    /// match at a time while attached to Python.
    fn refill(
        &mut self,
        py: Python<'_>,
        ac_impl: &AhoCorasick,
        cursor: &mut MatchCursor,
        batch_size: usize,
    ) -> Result<Option<Vec<(usize, u64)>>, MatchError> {
        let batch = match self {
            Self::String {
                haystack,
                end_code_point,
            } => {
                let haystack = haystack.as_str();
                py.detach(|| {
                    cursor.collect_batch(
                        ac_impl,
                        haystack.as_bytes(),
                        batch_size,
                        |previous_end_byte, mat| {
                            code_point_match_pair(haystack, previous_end_byte, end_code_point, mat)
                        },
                    )
                })?
            }
            Self::Bytes(haystack) => {
                let haystack = haystack.as_bytes(py);
                py.detach(|| {
                    cursor
                        .collect_batch(ac_impl, haystack, batch_size, |_, mat| byte_match_pair(mat))
                })?
            }
            Self::Buffer { .. } => return Ok(None),
        };
        Ok(Some(batch))
    }
}

#[inline]
fn byte_match_pair(mat: Match) -> (usize, u64) {
    (mat.end(), mat.pattern().as_u64())
}

#[inline]
fn code_point_match_pair(
    haystack: &str,
    previous_end_byte: usize,
    end_code_point: &mut usize,
    mat: Match,
) -> (usize, u64) {
    // Match ends never move backward, even when starts overlap.
    let end_byte = mat.end();
    let byte_delta = end_byte - previous_end_byte;
    // A single byte between UTF-8 boundaries is one code point.
    *end_code_point += if byte_delta <= 1 {
        byte_delta
    } else {
        haystack[previous_end_byte..end_byte].chars().count()
    };
    (*end_code_point, mat.pattern().as_u64())
}

/// Lazy Python iterator over match indexes.
#[pyclass]
struct PyMatchIterator {
    ac_impl: AhoCorasick,
    haystack: Option<MatchHaystack>,
    cursor: MatchCursor,
    iter_batch_size: NonZeroUsize,
    buffered: std::vec::IntoIter<(usize, u64)>,
    scanned_to_end: bool,
}

impl PyMatchIterator {
    fn new(
        ac_impl: AhoCorasick,
        haystack: MatchHaystack,
        overlapping: bool,
        iter_batch_size: NonZeroUsize,
    ) -> PyResult<Self> {
        let cursor = MatchCursor::new(&ac_impl, overlapping).map_err(match_error_to_pyerror)?;
        Ok(Self {
            ac_impl,
            haystack: Some(haystack),
            cursor,
            iter_batch_size,
            buffered: Vec::new().into_iter(),
            scanned_to_end: false,
        })
    }
}

#[pymethods]
impl PyMatchIterator {
    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<(usize, u64)>> {
        if let Some(pair) = self.buffered.next() {
            return Ok(Some(pair));
        }
        if self.scanned_to_end {
            self.buffered = Vec::new().into_iter();
            self.haystack = None;
            return Ok(None);
        }
        let Some(haystack) = &mut self.haystack else {
            return Ok(None);
        };

        // The constructors reject empty patterns, so a positive end offset
        // means the first match has already been returned to Python.
        let batch_size = self.iter_batch_size.get();
        if batch_size > 1
            && self.cursor.end_byte > 0
            && let Some(batch) = haystack
                .refill(py, &self.ac_impl, &mut self.cursor, batch_size)
                .map_err(match_error_to_pyerror)?
        {
            self.scanned_to_end = batch.len() < batch_size;
            self.buffered = batch.into_iter();
            let pair = self.buffered.next();
            if pair.is_none() {
                self.haystack = None;
            }
            return Ok(pair);
        }

        if let Some(mat) = self.cursor.next_pending(&self.ac_impl) {
            // Pending matches share the previous end and need no haystack access.
            debug_assert_eq!(mat.end(), self.cursor.end_byte);
            return Ok(Some(match haystack {
                MatchHaystack::String { end_code_point, .. } => {
                    (*end_code_point, mat.pattern().as_u64())
                }
                MatchHaystack::Bytes(_) | MatchHaystack::Buffer { .. } => byte_match_pair(mat),
            }));
        }

        let previous_end_byte = self.cursor.end_byte;
        let mut search = |haystack: &[u8]| self.cursor.scan(&self.ac_impl, haystack);
        let found = match haystack {
            MatchHaystack::String {
                haystack,
                end_code_point,
            } => {
                let haystack = haystack.as_str();
                py.detach(|| {
                    Ok(search(haystack.as_bytes())?.map(|mat| {
                        code_point_match_pair(haystack, previous_end_byte, end_code_point, mat)
                    }))
                })
            }
            MatchHaystack::Bytes(haystack) => {
                let haystack = haystack.as_bytes(py);
                py.detach(|| search(haystack).map(|mat| mat.map(byte_match_pair)))
            }
            MatchHaystack::Buffer { buffer, .. } => {
                let haystack = buffer_bytes(py, buffer);
                // Finding one match is cheaper than detaching from the runtime.
                search(haystack).map(|mat| mat.map(byte_match_pair))
            }
        }
        .map_err(match_error_to_pyerror)?;

        if found.is_none() {
            self.haystack = None;
        }
        Ok(found)
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        match &self.haystack {
            Some(MatchHaystack::String { haystack, .. }) => visit.call(haystack.as_py_str()),
            Some(MatchHaystack::Bytes(haystack)) => visit.call(haystack),
            Some(MatchHaystack::Buffer { view, .. }) => {
                // `view` is referenced twice: by us, and by `buffer`, whose
                // `Py_buffer.obj` is a strong reference to the memoryview it
                // was acquired from. GC needs both to see the cycle.
                visit.call(view)?;
                visit.call(view)
            }
            None => Ok(()),
        }
    }

    fn __clear__(&mut self) {
        self.buffered = Vec::new().into_iter();
        self.haystack = None;
    }
}

/// Methods for PyBytesAhoCorasick.
#[pymethods]
impl PyBytesAhoCorasick {
    /// __new__() implementation.
    #[new]
    #[pyo3(signature = (patterns, matchkind = PyMatchKind::Standard, implementation = None, *, iter_batch_size = 128))]
    fn new(
        _py: Python,
        patterns: Bound<'_, PyAny>,
        matchkind: PyMatchKind,
        implementation: Option<Implementation>,
        iter_batch_size: usize,
    ) -> PyResult<Self> {
        let iter_batch_size = validate_iter_batch_size(iter_batch_size)?;
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

        Ok(Self {
            ac_impl,
            iter_batch_size,
        })
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

    /// Return a lazy iterator of ``(end_index_in_haystack,
    /// index_into_patterns)`` tuples, in the same order as
    /// ``find_matches_as_indexes()``.
    ///
    /// For built-in bytes, the iterator may search ahead after the first
    /// match according to ``iter_batch_size`` on the matcher.
    ///
    /// Do not mutate a mutable buffer while the iterator is in use.
    #[pyo3(signature = (haystack, overlapping = false))]
    fn find_matches_as_indexes_iter(
        self_: Py<Self>,
        py: Python<'_>,
        haystack: Bound<'_, PyAny>,
        overlapping: bool,
    ) -> PyResult<Py<PyMatchIterator>> {
        let haystack = match haystack.cast_into_exact::<PyBytes>() {
            Ok(bytes) => MatchHaystack::Bytes(bytes.unbind()),
            Err(other) => {
                // Subclasses and other exporters can provide their own buffers.
                let view = PyMemoryView::from(&other.into_inner())?;
                let buffer = get_byte_buffer(view.as_any())?;
                MatchHaystack::Buffer {
                    view: view.unbind(),
                    buffer,
                }
            }
        };
        let (ac_impl, iter_batch_size) = {
            let matcher = self_.bind(py).borrow();
            (matcher.ac_impl.clone(), matcher.iter_batch_size)
        };
        let iterator = PyMatchIterator::new(ac_impl, haystack, overlapping, iter_batch_size)?;
        Py::new(py, iterator)
    }
}

/// The main Python module.
#[pymodule(gil_used = false)]
fn ahocorasick_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMatchKind>()?;
    m.add_class::<Implementation>()?;
    m.add_class::<PyAhoCorasick>()?;
    m.add_class::<PyBytesAhoCorasick>()?;
    m.add_class::<PyMatchIterator>()?;
    Ok(())
}

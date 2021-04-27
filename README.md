# ahocorasick_rs: Quickly search for multiple substrings at once

`ahocorasick_rs` allows you to search for multiple substrings ("patterns") in a given string ("haystack") using variations of the [Aho-Corasick algorithm](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm).

In particular, it's implemented as a wrapper of the Rust [`aho-corasick`](https://docs.rs/aho-corasick/) library, and provides a (sometimes) faster alternative to the [`pyahocorasick`](https://pyahocorasick.readthedocs.io/) library.

The specific use case is searching for large numbers of patterns (in the thousands) where the Rust library's DFA-based state machine allows for faster matching.

* [Quickstart](#quickstart)
* [Additional configuration](#configuration)
* [Implementation details](#implementation)
* [Benchmarks](#benchmarks)

## Quickstart <a name="quickstart"></a>

The `ahocorasick_rs` library allows you to search for multiple strings ("patterns") within a haystack.
For example, let's construct a `AhoCorasick` object:

```python
>>> import ahocorasick_rs
>>> patterns = ["hello", "world", "fish"2]
>>> haystack = "this is my first hello world. hello!"
>>> ac = ahocorasick_rs.AhoCorasick(patterns)
```

`AhoCorasick.find_matches_as_indexes()` returns a list of tuples, each tuple being:

1. The index of the found pattern inside the list of patterns.
2. The start index of the pattern inside the haystack.
3. The end index of the pattern inside the haystack.

```python
>>> ac.find_matches_as_indexes(haystack)
[(0, 17, 22), (1, 23, 28), (0, 30, 35)]
>>> patterns[0], patterns[1], patterns[0]
('hello', 'world', 'hello')
>>> haystack[17:22], haystack[23:28], haystack[30:35]
('hello', 'world', 'hello')
```

`find_matches_as_strings()` returns a list of found patterns:

```python
>>> ac.find_matches_as_strings(haystack)
['hello', 'world', 'hello']
```

## Additional configuration <a name="configuration"></a>

### Match kind

There are three ways you can configure matching in cases where multiple patterns overlap.
Assume we have this starting point:

```python
>>> from ahocorasick_rs import *
>>> patterns = ["disco", "disc", "discontent"]
>>> haystack = "discontent"
```

#### `MATCHKIND_STANDARD` (the default)

This returns the pattern that matches first, semantically-speaking.

```python
>>> AhoCorasick(patterns).find_matches_as_strings(haystack)
['disc']
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)
>>> ac.find_matches_as_strings(haystack)
['disc']
```

In this case `disc` will match before `disco` or `discontent`.

#### `MATCHKIND_LEFTMOST_FIRST`

This returns the leftmost matching pattern that appears first in the list of patterns.

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)
>>> ac.find_matches_as_strings(haystack)
['disco']
```

`disco` is returned because it precedes `disc` in the list of patterns.

##### `MATCHKIND_LEFTMOST_LONGEST`

This returns the leftmost matching pattern that is longest:

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)
>>> ac.find_matches_as_strings(haystack)
['discontent']
```

### Overlapping matches

You can get all overlapping matches, instead of just one of them, but only if you stick to the default matchkind, `MATCHKIND_STANDARD`:

```python
>>> from ahocorasick_rs import AhoCorasick
>>> patterns = ["winter", "onte", "disco", "discontent"]
>>> ac = AhoCorasick(patterns)
>>> ac.find_matches_as_strings("discontent", overlapping=True)
['disco', 'onte', 'discontent']
```

## Implementation details <a name="implementation"></a>

* The underlying Rust library supports two implementations, one oriented towards reducing memory usage and construction time (NFA), the latter towards faster matching (DFA).
  The Python wrapper only exposes the DFA version, since expensive setup compensated by fast batch operations is the standard Python tradeoff.
* Matching releases the GIL, to enable concurrency.
* Not all features from the underlying library are exposed; if you would like additional features, please [file an issue](https://github.com/g-research/ahocorasick_rs/issues/new) or submit a PR.

## Benchmarks <a name="benchmarks"></a>

Based on `benchmarks/test_comparison.py`, the benchmark matches ~1,000 patterns against 10,000 lines of text, with (some of?) the benchmarking overhead subtracted.

Lower is better: for longest matching pattern, `ahocorasick_rs` is faster.
For overlapping matches, `pyahocorasick` is faster.

| `find_matches_as_strings` or equivalent | milliseconds per 10K |
|-----------------------------------------|---------------------:|
| `ahocorasick_rs` standard matching      |                 6.12 |
| `ahocorasick_rs` longest matching       |                 6.89 |
| `pyahocorasick` longest matching        |                 8.97 |
| `ahocorasick_rs` overlapping matching   |                14.96 |
| `pyahocorasick` overlapping matching    |                11.38 |

> **Important:** As with any benchmark, real-world results will differ based on your particular situation. If performance is important to your application, measure the alternatives yourself!

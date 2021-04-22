# Python library wrapping https://github.com/BurntSushi/aho-corasick

TODO What is Aho-Corasick?

The Rust Aho-Corasick library is significantly faster than `pyahocorasick`.
This wrapper makes it accessible from Python.

## Quickstart

The `ahocorasick_rs` library allows you to search for multiple strings ("patterns") within a haystack.
For example:

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

## Additional configuration

### Match kind

There three ways you can configure matching in cases where multiple patterns overlap.
Assuming we have this starting point:

```python
>>> from ahocorasick_rs import *
>>> patterns = ["disco", "disc", "discontent"]
>>> haystack = "discontent"
```

#### `MATCHKIND_STANDARD` (the default)

This returns the first one that matches.

```python
>>> AhoCorasick(patterns).find_matches_as_strings(haystack)
['disc']
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)
>>> ac.find_matches_as_strings(haystack)
['disc']
```

#### `MATCHKIND_LEFTMOST_FIRST`

This returns the leftmost matching pattern that appears first in the list of patterns.

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)
>>> ac.find_matches_as_strings(haystack)
['disco']
```

##### `MATCHKIND_LEFTMOST_LONGEST`

This returns the leftmost matching pattern that is longest:

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)
>>> ac.find_matches_as_strings(haystack)
['discontent']
```

### Overlapping matches

You can get all overlapping matches, instead of just one of them, but only if you stick to the default matchkind, MATCHKIND_STANDARD:

```python
>>> from ahocorasick_rs import AhoCorasick
>>> patterns = ["winter", "onte", "disco", "discontent"]
>>> ac = AhoCorasick(patterns)
>>> ac.find_matches_as_strings("discontent", overlapping=True)
['disco', 'onte', 'discontent']
```

## Implementation details

* The underlying library supports two implementations, one oriented towards reducing memory usage and construction time (NFA), the latter towards faster matching (DFA).
  The Python wrapper only exposes the DFA version, since expensive setup compensated by fast batch operations is the standard Python tradeoff.
* Matching releases the GIL, to enable concurrency.
* Not all features from the underlying library are exposed; if you would like additional features, please [file an issue](https://github.com/g-research/ahocorasick_rs/issues/new) or submit a PR.

## Benchmarks

This is what gets benchmarked in `benchmarks/test_comparison.py`, matching ~1000 patterns against a line of text, with the benchmarked overhead subtracted.

As you can see, for longest matching pattern, `ahocorasick_rs` is faster. For overlapping matches, `pyahocorasick` is faster. Lower is better:

| `find_matches_as_strings` or equivalent | nanoseconds per call |
|-----------------------------------------|---------------------:|
| `ahocorasick_rs` standard matching      |                  576 |
| `ahocorasick_rs` longest matching       |                  633 |
| `pyahocorasick` longest matching        |                  791 |
| `ahocorasick_rs` overlapping matching   |                 1362 |
| `pyahocorasick` overlapping matching    |                 1139 |

> **Important:** As with any benchmark, real-world results will differ based on your particular situation. If performance is important to your application, measure the alternatives yourself!

## Features to implement

For each feature, include tests and documentation in README.

* [x] Basic API
* [x] Match kind
* [x] Overlapping
* [x] DFA
* [x] Release GIL
* [x] Benchmarks
* [ ] Batch mode for Pandas columns
* [ ] Finish documentation (README) - link to underlying library, explain what the library does, its performance goals, initial benchmark results
* [ ] GitHub Actions testing setup
* [ ] PyPI release automation
* [ ] Initial release

Other features in API: open issues, they seem less useful.

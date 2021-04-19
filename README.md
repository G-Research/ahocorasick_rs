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

`find_matches_as_strings()` returns a list of tuples, each tuple being:

1. The pattern.
2. Its start index inside the haystack (the end index can be derived by adding the length of the pattern).

```python
>>> ac.find_matches_as_strings(haystack)
[('hello', 17), ('world', 23), ('hello', 30)]
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
[('disc', 0)]
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)
>>> ac.find_matches_as_strings(haystack)
[('disc', 0)]
```

#### `MATCHKIND_LEFTMOST_FIRST`

This returns the leftmost matching pattern that appears first in the list of patterns.

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)
>>> ac.find_matches_as_strings(haystack)
[('disco', 0)]
```

##### `MATCHKIND_LEFTMOST_LONGEST`

This returns the leftmost matching pattern that is longest:

```python
>>> ac = AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)
>>> ac.find_matches_as_strings(haystack)
[('discontent', 0)]
```

### Overlapping matches

You can get all overlapping matches, instead of just one of them, but only if you stick to the default matchkind, MATCHKIND_STANDARD:

```python
>>> from ahocorasick_rs import AhoCorasick
>>> patterns = ["winter", "onte", "disco", "discontent"]
>>> ac = AhoCorasick(patterns)
>>> ac.find_matches_as_strings("discontent", overlapping=True)
[('disco', 0), ('onte', 4), ('discontent', 0)]
```

## TODO Benchmarks

## Features to implement

For each feature, include tests and documentation in README.

* [x] Basic API
* [x] Match kind
* [x] Overlapping
* [ ] DFA
* [ ] ascii case insensitive
* [ ] Finish documentation (README)
* [ ] Maturin builds in GitHub Actions
* [ ] PyPI release

Other features in API: open issues, they seem less useful.

# ahocorasick_rs: Quickly search for multiple substrings at once

`ahocorasick_rs` allows you to search for multiple substrings ("patterns") in a given string ("haystack") using variations of the [Aho-Corasick algorithm](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm).

In particular, it's implemented as a wrapper of the Rust [`aho-corasick`](https://docs.rs/aho-corasick/) library, and provides a faster alternative to the [`pyahocorasick`](https://pyahocorasick.readthedocs.io/) library.

The specific use case is searching for large numbers of patterns (in the thousands) where the Rust library's DFA-based state machine allows for faster matching.

Found any problems or have any questions? [File an issue on the GitHub project](https://github.com/G-Research/ahocorasick_rs).

* [Quickstart](#quickstart)
* [Additional configuration](#configuration)
* [Implementation details](#implementation)
* [Benchmarks](#benchmarks)

## Quickstart <a name="quickstart"></a>

The `ahocorasick_rs` library allows you to search for multiple strings ("patterns") within a haystack.
For example, let's install the library:

```shell-session
$ pip install ahocorasick-rs
```

Then, we can construct a `AhoCorasick` object:

```python
>>> import ahocorasick_rs
>>> patterns = ["hello", "world", "fish"]
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
For a more in-depth explanation, see the [underlying Rust library's documentation of matching](https://docs.rs/aho-corasick/latest/aho_corasick/enum.MatchKind.html).

Assume we have this starting point:

```python
>>> from ahocorasick_rs import AhoCorasick, MatchKind
```

#### `Standard` (the default)

This returns the pattern that matches first, semantically-speaking.
This is the default matching pattern.

```python
>>> ac AhoCorasick(["disco", "disc", "discontent"])
>>> ac.find_matches_as_strings("discontent")
['disc']
>>> ac = AhoCorasick(["b", "abcd"])
>>> ac.find_matches_as_strings("abcdef")
['b']
```

In this case `disc` will match before `disco` or `discontent`.

Similarly, `b` will match before `abcd` because it ends earlier in the haystack than `abcd` does:

```python
>>> ac = AhoCorasick(["b", "abcd"])
>>> ac.find_matches_as_strings("abcdef")
['b']
```

#### `LeftmostFirst`

This returns the leftmost-in-the-haystack matching pattern that appears first in _the list of given patterns_.
That means the order of patterns makes a difference:

```python
>>> ac = AhoCorasick(["disco", "disc"], matchkind=MatchKind.LeftmostFirst)
>>> ac.find_matches_as_strings("discontent")
['disco']
>>> ac = AhoCorasick(["disc", "disco"], matchkind=MatchKind.LeftmostFirst)
['disc']
```

Here we see `abcd` matched first, because it starts before `b`:

```python
>>> ac = AhoCorasick(["b", "abcd"], matchkind=MatchKind.LeftmostFirst)
>>> ac.find_matches_as_strings("abcdef")
['abcd']
```
##### `LeftmostLongest`

This returns the leftmost-in-the-haystack matching pattern that is longest:

```python
>>> ac = AhoCorasick(["disco", "disc", "discontent"], matchkind=MatchKind.LeftmostLongest)
>>> ac.find_matches_as_strings("discontent")
['discontent']
```

### Overlapping matches

You can get all overlapping matches, instead of just one of them, but only if you stick to the default matchkind, `MatchKind.Standard`:

```python
>>> from ahocorasick_rs import AhoCorasick
>>> patterns = ["winter", "onte", "disco", "discontent"]
>>> ac = AhoCorasick(patterns)
>>> ac.find_matches_as_strings("discontent", overlapping=True)
['disco', 'onte', 'discontent']
```

### Trading memory for speed

If you use ``find_matches_as_strings()``, there are two ways strings can be constructed: from the haystack, or by caching the patterns on the object.
The former takes more work, the latter uses more memory if the patterns would otherwise have been garbage-collected.
You can control the behavior by using the `store_patterns` keyword argument to `AhoCorasick()`.

* ``AhoCorasick(..., store_patterns=None)``: The default.
  Use a heuristic (currently, whether the total of pattern string lengths is less than 4096 characters) to decide whether to store patterns or not.
* ``AhoCorasick(..., store_patterns=True)``: Keep references to the patterns, potentially speeding up ``find_matches_as_strings()`` at the cost of using more memory.
  If this uses large amounts of memory this might actually slow things down due to pressure on the CPU memory cache, and/or the performance benefit might be overwhelmed by the algorithm's search time.
* ``AhoCorasick(..., store_patterns=False)``: Don't keep references to the patterns, saving some memory but potentially slowing down ``find_matches_as_strings()``, especially when there are only a small number of patterns and you are searching a small haystack.

### Algorithm implementations: trading construction speed, memory, and performance

You can choose the type of underlying automaton to use, with different performance tradeoffs.

The underlying Rust library supports [four choices](https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasickBuilder.html#method.kind), which are exposed:

* `None` uses a heuristic to choose the "best" Aho-Corasick implementation for the given patterns.
* `Implementation.NoncontiguousNFA`: A noncontiguous NFA is the fastest to be built, has moderate memory usage and is typically the slowest to execute a search.
* `Implementation.ContiguousNFA`: A contiguous NFA is a little slower to build than a noncontiguous NFA, has excellent memory usage and is typically a little slower than a DFA for a search.
* `Implementation.DFA`: A DFA is very slow to build, uses exorbitant amounts of memory, but will typically execute searches the fastest.

The default choice is `Implementation.DFA` since expensive setup compensated by fast batch operations is the standard Python tradeoff.

```python
>>> from ahocorasick_rs import AhoCorasick, Implementation
>>> ac = AhoCorasick(["disco", "disc"], implementation=Implementation.NoncontiguousNFA)
```

## Implementation details <a name="implementation"></a>

* Matching releases the GIL, to enable concurrency.
* Not all features from the underlying library are exposed; if you would like additional features, please [file an issue](https://github.com/g-research/ahocorasick_rs/issues/new) or submit a PR.

## Benchmarks <a name="benchmarks"></a>

As with any benchmark, real-world results will differ based on your particular situation.
If performance is important to your application, measure the alternatives yourself!

### Longer strings and many patterns

This benchmark matches ~4,000 patterns against lines of text that are ~700 characters long.
Each line matches either zero (90%) or one pattern (10%).

Higher is better; `ahocorasick_rs` is much faster in both cases.

| `find_matches_as_strings` or equivalent | Operations per second |
|-----------------------------------------|---------------------:|
| `ahocorasick_rs` longest matching       |            `436,000` |
| `pyahocorasick` longest matching        |             `65,000` |
| `ahocorasick_rs` overlapping matching   |            `329,000` |
| `pyahocorasick` overlapping matching    |             `76,000` |

### Shorter strings and few patterns

This benchmarks matches ~10 patterns against lines of text that are ~70 characters long.
Each line matches ~5 patterns.

Higher is better; again, `ahocorasick_rs` is faster for both, though with a smaller margin.

| `find_matches_as_strings` or equivalent | Operations per second   |
|-----------------------------------------|------------------------:|
| `ahocorasick_rs` longest matching       |             `1,930,000` |
| `pyahocorasick` longest matching        |             `1,120,000` |
| `ahocorasick_rs` overlapping matching   |             `1,250,000` |
| `pyahocorasick` overlapping matching    |               `880,000` |


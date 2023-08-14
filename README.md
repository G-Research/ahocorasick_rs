# ahocorasick_rs: Quickly search for multiple substrings at once

`ahocorasick_rs` allows you to search for multiple substrings ("patterns") in a given string ("haystack") using variations of the [Aho-Corasick algorithm](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm).

In particular, it's implemented as a wrapper of the Rust [`aho-corasick`](https://docs.rs/aho-corasick/) library, and provides a faster alternative to the [`pyahocorasick`](https://pyahocorasick.readthedocs.io/) library.

Found any problems or have any questions? [File an issue on the GitHub project](https://github.com/G-Research/ahocorasick_rs).

* [Quickstart](#quickstart)
* [Choosing the matching algorithm](#matching)
* [Additional configuration: speed and memory usage tradeoffs](#configuration2)
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

You can construct a `AhoCorasick` object from any iterable (including generators), not just lists:

```python
>>> ac = ahocorasick_rs.AhoCorasick((p.lower() for p in patterns))
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

## Choosing the matching algorithm <a name="matching"></a>

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

## Additional configuration: speed and memory usage tradeoffs <a name="configuration2"></a>

### Algorithm implementations: trading construction speed, memory, and performance

You can choose the type of underlying automaton to use, with different performance tradeoffs.
The short version: if you want maximum matching speed, and you don't have too many patterns, try the `Implementation.DFA` implementation and see if it helps.

The underlying Rust library supports [four choices](https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasickBuilder.html#method.kind), which are exposed as follows:

* `None` uses a heuristic to choose the "best" Aho-Corasick implementation for the given patterns, balancing construction time, memory usage, and matching speed.
  This is the default.
* `Implementation.NoncontiguousNFA`: A noncontiguous NFA is the fastest to be built, has moderate memory usage and is typically the slowest to execute a search.
* `Implementation.ContiguousNFA`: A contiguous NFA is a little slower to build than a noncontiguous NFA, has excellent memory usage and is typically a little slower than a DFA for a search.
* `Implementation.DFA`: A DFA is very slow to build, uses exorbitant amounts of memory, but will typically execute searches the fastest.

```python
>>> from ahocorasick_rs import AhoCorasick, Implementation
>>> ac = AhoCorasick(["disco", "disc"], implementation=Implementation.DFA)
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

## Implementation details <a name="implementation"></a>

* Matching releases the GIL, to enable concurrency.
* Not all features from the underlying library are exposed; if you would like additional features, please [file an issue](https://github.com/g-research/ahocorasick_rs/issues/new) or submit a PR.

## Benchmarks <a name="benchmarks"></a>

As with any benchmark, real-world results will differ based on your particular situation.
If performance is important to your application, measure the alternatives yourself!

That being said, I've seen `ahocorasick_rs` run 1.5× to 7× as fast as `pyahocorasick`, depending on the options used.
You can run the included benchmarks, if you want, to see some comparative results locally.
Clone the repository, then:

```
pip install pytest-benchmark ahocorasick_rs pyahocorasick
pytest benchmarks/
```

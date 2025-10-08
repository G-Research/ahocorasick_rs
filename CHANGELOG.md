# Changelog

## 1.0

* Added support for Python 3.14 and free-threaded Python.
* In order to support free-threaded Python, move the burden of thread safety onto users.
  Specifically: you must not mutate byte arrays and the like that are passed to `BytesAhoCorasick` APIs while those APIs are running.
* Dropped support for Python 3.9.

## 0.22.2

* Update Rust dependencies.

## 0.22.1

* Added support for Python 3.13.
* Dropped support for Python 3.8.

## 0.22.0

* The GIL is released when using a `bytes` haystack with `BytesAhoCorasick`. Thanks to Isaac Garzón.

## 0.21.0

* Added support for searching `bytes`, `bytearray`, `memoryview`, and similar objects using the `BytesAhoCorasick` class. Thanks to Isaac Garzón.

## 0.20.0

* Added support for Python 3.12.

## 0.19.0

* If an empty string is passed in as a pattern, `AhoCorasick()` will now raise a `ValueError`.
  Previously using empty patterns could result in garbage results or exceptions when matching.
* Upgraded to `aho-corasick` v1.1.1.

## 0.18.0

* Upgraded to `aho-corasick` v1.1.0, which can run faster on ARM machines like newer Macs.

## 0.17.0

* Upgraded to `aho-corasick` v1.0.5, fixing performance regression in construction of `AhoCorasick` objects.

## 0.16.0

* Upgraded to `aho-corasick` v1.0.4, leading to massive reduction in memory usage during construction.

## 0.15.0

* `AhoCorasick()` can now accept an iterable of strings, instead of just a list of strings.
  This means you can, for example, lazily create patterns in a generator to save memory; the underlying implementation will still use the bulk of the memory, however.
* The default implementation (which used to be DFA) will now be chosen heuristically based on the inputs; this may result in a slow-down in some cases.
  To get the old behavior, you can do `AhoCorasick(..., implementation=Implementation.DFA)`.
* Upgraded to `aho-corasick` v1.0.3, which will result in lower memory usage in some cases.
* Dropped support for Python 3.7, which is now end of life.
* Dropped support for macOS 10.5, which is now end of life.

## 0.14.0

* Upgraded to `aho-corasick` v1.0.
* Exposed the choice between DFA, non-contiguous NFA, and contiguous NFA implementations.
* `ahocorasick.MatchKind` enum replaces the module-level `MATCHKIND_*` constants.
  (The old `MATCHKIND_*` constants are deprecated, but will continue to work so long as you were using constants rather than strings.)
* Add type signatures.

## 0.13.0

* Use less memory when constructing an instance.
* Added an option (``store_patterns``) to control whether patterns are cached on the object.
  The new behavior is to use a heuristic by default to decide whether to cache the strings.

## 0.12.3

* Added support for Python 3.11.

## 0.12.2

* Added wheels for ARM Macs, and Linux on ARM.
* Started distributing a source tarball for platforms lacking wheels.

## 0.12.1

* Dropped support for Python 3.6 (it is no longer maintained); users of Python 3.6 can still use older releases.
* Improved performance a little.

## 0.12.0

* Added support for Python 3.10.

## 0.11.0

* Faster performance, thanks to `PyO3` v0.14.

## 0.10.0

* Fixed bug where `find_matches_as_indexes()` didn't give correct offsets for
  non-ASCII strings
  ([#12](https://github.com/G-Research/ahocorasick_rs/issues/12)). Thanks to
  @necrosovereign for reporting and @BurntSushi for suggesting fix.

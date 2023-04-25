# Changelog

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

# Changelog

## 0.12.3

* Added support for Python 3.11 on x86-64 and macOS ARM (built against `rc1` for now).
  Linux ARM will arrive once 3.11 is released and relevant infrastructure is easier to use.

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

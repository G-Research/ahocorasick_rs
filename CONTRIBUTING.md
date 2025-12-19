# Contributing to `ahocorasick_rs`

Thank you for considering contributing! Note that by contributing your code, you are agreeing to license it under the Apache License 2.0 (see [`LICENSE`](LICENSE)).

## Setting up a development environment

First, download and install [Just](https://github.com/casey/just)

Then, setup a virtualenv, install dev dependencies, and compile the code:

```bash
just setup
source venv/bin/activateo  # or Windoes equivalent
just build-dev
```

## Running tests and benchmarks

To run tests:

```shell-session
$ just test
```

To run benchmarks:

```shell-session
$ just prep-benchmark  # on Linux Intel, disables turbobost
$ just benchmark
```

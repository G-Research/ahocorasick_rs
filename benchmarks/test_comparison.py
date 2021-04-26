"""
Benchmarks comparing ahocorasick_rs to other libraries.

We don't just use the same string each time, because pyo3 wants UTF-8, and
Python's UTF-8 conversion API caches the conversion, so repeatedly using the
same haystack would understate ahocorasick_rs's overhead.
"""

import random

import ahocorasick
import ahocorasick_rs

PATTERNS = [
    "abc",
    "hello",
    "world",
    "aardvark",
    "fish",
    "what",
    "arbitrarymonkey",
    "birds",
]
PATTERNS += ["host%d" % i for i in range(500)]
PATTERNS += [str(random.random()) for i in range(500)]

HAYSTACK = "arbitrarymonkey says hello to fish host76, 0.123 my friend, but why???"


def make_pyahocorasick_automaton():
    automaton = ahocorasick.Automaton()
    for key in PATTERNS:
        automaton.add_word(key, key)
    automaton.make_automaton()
    return automaton


def test_pyahocorasick_overlapping(benchmark):
    """pyahocorasick overlapping matches."""
    automaton = make_pyahocorasick_automaton()

    def run():
        for i in range(10000):
            x = list(automaton.iter(HAYSTACK + str(i)))
        return x

    print(benchmark(run))


def test_pyahocorasick_longest_match(benchmark):
    """pyahocorasick longest matches."""
    automaton = make_pyahocorasick_automaton()

    def run():
        for i in range(10000):
            x = list(automaton.iter_long(HAYSTACK + str(i)))
        return x

    print(benchmark(run))


def test_ahocorasick_rs_standard(benchmark):
    """ahocorasick_rs standard matching algorithm."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for i in range(10000):
            x = ac.find_matches_as_strings(HAYSTACK + str(i))
        return x

    print(benchmark(run))


def test_ahocorasick_rs_standard_indexes(benchmark):
    """ahocorasick_rs standard matching algorithm, returning indexes."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for i in range(10000):
            x = ac.find_matches_as_indexes(HAYSTACK + str(i))
        return x

    print(benchmark(run))


def test_ahocorasick_rs_overlapping(benchmark):
    """ahocorasick_rs overlapping matches."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for i in range(10000):
            x = ac.find_matches_as_strings(HAYSTACK + str(i), overlapping=True)
        return x

    print(benchmark(run))


def test_ahocorasick_rs_longest_match(benchmark):
    """ahocorasick_rs longest matches."""
    ac = ahocorasick_rs.AhoCorasick(
        PATTERNS, matchkind=ahocorasick_rs.MATCHKIND_LEFTMOST_LONGEST
    )

    def run():
        for i in range(10000):
            x = ac.find_matches_as_strings(HAYSTACK + str(i))
        return x

    print(benchmark(run))


def test_overhead(benchmark):
    """Just run a function that does everything other than call API."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for i in range(10000):
            _ = HAYSTACK + str(i)

    print(benchmark(run))

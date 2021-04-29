"""
Benchmarks comparing ahocorasick_rs to other libraries.

We don't just use the same string each time, because pyo3 wants UTF-8, and
Python's UTF-8 conversion API caches the conversion, so repeatedly using the
same haystack would understate ahocorasick_rs's overhead.
"""

import random
import os

import ahocorasick
import ahocorasick_rs

# ~5000 popular first names, filtered down to ~4200:
with open(os.path.join(os.path.dirname(__file__), "names.txt")) as f:
    PATTERNS = [line.strip().lower() for line in f if len(line.strip()) > 4]


# 90% no matches, 10% with 1 match, about 70 characters
def make_haystacks():
    line = "No one who had ever seen {} in her infancy would have supposed her born to be an heroine. Her situation in life, the character of her father and mother, her own person and disposition, were all equally against her. Her father was a clergyman, without being neglected, or poor, and a very respectable man, though his name was whatevs—and he had never been handsome. He had a considerable independence besides two good livings—and he was not in the least addicted to locking up his daughters. Her mother was a woman of useful plain sense, with a good temper, and, what is more remarkable, with a good constitution.".lower()
    result = []
    for i in range(100000):
        if i % 90 == 0:
            name = PATTERNS[i % len(PATTERNS)]
        else:
            name = "notaperson"
        result.append(line.format(name))
    return result


HAYSTACKS = make_haystacks()


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
        for haystack in HAYSTACKS:
            x = list(automaton.iter(haystack))
        return x

    print(benchmark(run))


def test_pyahocorasick_longest_match(benchmark):
    """pyahocorasick longest matches."""
    automaton = make_pyahocorasick_automaton()

    def run():
        for haystack in HAYSTACKS:
            x = list(automaton.iter_long(haystack))
        return x

    print(benchmark(run))


def test_ahocorasick_rs_standard(benchmark):
    """ahocorasick_rs standard matching algorithm."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for haystack in HAYSTACKS:
            x = ac.find_matches_as_strings(haystack)
        return x

    print(benchmark(run))


def test_ahocorasick_rs_standard_indexes(benchmark):
    """ahocorasick_rs standard matching algorithm, returning indexes."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for haystack in HAYSTACKS:
            x = ac.find_matches_as_indexes(haystack)
        return x

    print(benchmark(run))


def test_ahocorasick_rs_overlapping(benchmark):
    """ahocorasick_rs overlapping matches."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for haystack in HAYSTACKS:
            x = ac.find_matches_as_strings(haystack, overlapping=True)
        return x

    print(benchmark(run))


def test_ahocorasick_rs_longest_match(benchmark):
    """ahocorasick_rs longest matches."""
    ac = ahocorasick_rs.AhoCorasick(
        PATTERNS, matchkind=ahocorasick_rs.MATCHKIND_LEFTMOST_LONGEST
    )

    def run():
        for haystack in HAYSTACKS:
            x = ac.find_matches_as_strings(haystack)
        return x

    print(benchmark(run))


def test_overhead(benchmark):
    """Just run a function that does everything other than call API."""
    ac = ahocorasick_rs.AhoCorasick(PATTERNS)

    def run():
        for haystack in HAYSTACKS:
            _ = haystack

    print(benchmark(run))

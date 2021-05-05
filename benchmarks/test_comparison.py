"""
Benchmarks comparing ahocorasick_rs to other libraries.

We don't just use the same string each time, because pyo3 wants UTF-8, and
Python's UTF-8 conversion API caches the conversion, so repeatedly using the
same haystack would understate ahocorasick_rs's overhead.
"""

import os

import pytest

import ahocorasick  # pyahocorasick
import ahocorasick_rs  # our module

# ~5000 popular first names, filtered down to ~4200:
with open(os.path.join(os.path.dirname(__file__), "names.txt")) as f:
    PATTERNS_LONG = [line.strip().lower() for line in f if len(line.strip()) > 4]


# 90% no matches, 10% with 1 match, about 70 characters
def make_haystacks_long():
    line = "No one who had ever seen {} in her infancy would have supposed her born to be an heroine. Her situation in life, the character of her father and mother, her own person and disposition, were all equally against her. Her father was a clergyman, without being neglected, or poor, and a very respectable man, though his name was whatevs—and he had never been handsome. He had a considerable independence besides two good livings—and he was not in the least addicted to locking up his daughters. Her mother was a woman of useful plain sense, with a good temper, and, what is more remarkable, with a good constitution {}.".lower()
    result = []
    for i in range(100000):
        if i % 90 == 0:
            name = PATTERNS_LONG[i % len(PATTERNS_LONG)]
        else:
            name = "notaperson"
        result.append(line.format(name, i))
    return result


HAYSTACKS_LONG = make_haystacks_long()

PATTERNS_SHORT = [
    "abc",
    "hello",
    "world",
    "aardvark",
    "fish",
    "what",
    "arbitrarymonkey",
    "birds",
    "host7",
    "host76",
]
HAYSTACKS_SHORT = [
    "arbitrarymonkey says hello to fish host76, 0.123 my friend, but why??? {}".format(
        i
    )
    for i in range(10_000)
]


def make_pyahocorasick_automaton(patterns):
    automaton = ahocorasick.Automaton()
    for key in patterns:
        automaton.add_word(key, key)
    automaton.make_automaton()
    return automaton


parameterize_datasets = pytest.mark.parametrize(
    "test_data",
    [(PATTERNS_SHORT, HAYSTACKS_SHORT), (PATTERNS_LONG, HAYSTACKS_LONG)],
    ids=["short", "long"],
)


@parameterize_datasets
def test_pyahocorasick_overlapping(benchmark, test_data):
    """pyahocorasick overlapping matches."""
    patterns, haystacks = test_data
    automaton = make_pyahocorasick_automaton(patterns)

    def run():
        for haystack in haystacks:
            x = list(automaton.iter(haystack))
        return x

    print(benchmark(run))


@parameterize_datasets
def test_pyahocorasick_longest_match(benchmark, test_data):
    """pyahocorasick longest matches."""
    patterns, haystacks = test_data
    automaton = make_pyahocorasick_automaton(patterns)

    def run():
        for haystack in haystacks:
            x = list(automaton.iter_long(haystack))
        return x

    print(benchmark(run))


@parameterize_datasets
def test_ahocorasick_rs_standard(benchmark, test_data):
    """ahocorasick_rs standard matching algorithm."""
    patterns, haystacks = test_data
    ac = ahocorasick_rs.AhoCorasick(patterns)

    def run():
        for haystack in haystacks:
            x = ac.find_matches_as_strings(haystack)
        return x

    print(benchmark(run))


@parameterize_datasets
def test_ahocorasick_rs_standard_indexes(benchmark, test_data):
    """ahocorasick_rs standard matching algorithm, returning indexes."""
    patterns, haystacks = test_data
    ac = ahocorasick_rs.AhoCorasick(patterns)

    def run():
        for haystack in haystacks:
            x = ac.find_matches_as_indexes(haystack)
        return x

    print(benchmark(run))


@parameterize_datasets
def test_ahocorasick_rs_overlapping(benchmark, test_data):
    """ahocorasick_rs overlapping matches."""
    patterns, haystacks = test_data
    ac = ahocorasick_rs.AhoCorasick(patterns)

    def run():
        for haystack in haystacks:
            x = ac.find_matches_as_strings(haystack, overlapping=True)
        return x

    print(benchmark(run))


@parameterize_datasets
def test_ahocorasick_rs_longest_match(benchmark, test_data):
    """ahocorasick_rs longest matches."""
    patterns, haystacks = test_data
    ac = ahocorasick_rs.AhoCorasick(
        patterns, matchkind=ahocorasick_rs.MATCHKIND_LEFTMOST_LONGEST
    )

    def run():
        for haystack in haystacks:
            x = ac.find_matches_as_strings(haystack)
        return x

    print(benchmark(run))


@parameterize_datasets
def test_overhead(benchmark, test_data):
    """Just run a function that does everything other than call API."""
    _, haystacks = test_data

    def run():
        for haystack in haystacks:
            _ = haystack

    print(benchmark(run))

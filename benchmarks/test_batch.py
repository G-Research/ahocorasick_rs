"""
Benchmarks for batch operations.

Use case is a Pandas column, and wanting to match all strings in that column.
"""

import pandas as pd

import ahocorasick_rs

from .test_comparison import PATTERNS, HAYSTACK

AC = ahocorasick_rs.AhoCorasick(PATTERNS)
HAYSTACKS = [str(i) + HAYSTACK + str(i) for i in range(100_000)]


def test_list_sequential(benchmark):
    """Just do each line one at a time."""

    def run():
        l = []
        for h in HAYSTACKS:
            l.append(AC.find_matches_as_strings(h))
        return l

    result = benchmark(run)
    assert result == AC.parallel_map_to_strings(HAYSTACKS)


def test_list_in_parallel(benchmark):
    """Batch operation on a list of strings."""

    def run():
        AC.parallel_map_to_strings(HAYSTACKS)

    benchmark(run)


def test_pandas_apply(benchmark):
    """Pandas column with matching apply()ied."""
    series = pd.Series(HAYSTACKS)

    def run():
        return series.apply(AC.find_matches_as_strings)

    result = benchmark(run)
    assert result.equals(pd.Series(AC.parallel_map_to_strings(series.tolist())))


def test_pandas_in_parallel(benchmark):
    """Pandas column with matching in parallel"""
    series = pd.Series(HAYSTACKS)

    def run():
        pd.Series(AC.parallel_map_to_strings(series.values))

    benchmark(run)

"""
Benchmarks for batch operations.

Use case is a Pandas column, and wanting to match all strings in that column.
"""

import ahocorasick_rs

from .test_comparison import PATTERNS, HAYSTACK

AC = ahocorasick_rs.AhoCorasick(PATTERNS)
HAYSTACKS = [str(i) + HAYSTACK + str(i) for i in range(100_000)]


def test_no_batching(benchmark):
    """Just do each line one at a time."""

    def run():
        l = []
        for h in HAYSTACKS:
            l.append(AC.find_matches_as_strings(h))
        return l

    result = benchmark(run)
    assert result == AC.map_to_strings(HAYSTACKS)


def test_batch_list(benchmark):
    """Batch operation on a list of strings."""

    def run():
        AC.map_to_strings(HAYSTACKS)

    benchmark(run)

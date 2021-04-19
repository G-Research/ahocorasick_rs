"""Tests for ahocorasick_rs."""

import pytest

from ahocorasick_rs import (
    AhoCorasick,
    MATCHKIND_STANDARD,
    MATCHKIND_LEFTMOST_FIRST,
    MATCHKIND_LEFTMOST_LONGEST,
)


def test_basic_matching():
    """
    find_matches_as_indexes() and find_matches_as_indexes() return matching
    patterns in the given string.
    """
    haystack = "hello, world, hello again"
    patterns = ["hello", "world"]
    ac = AhoCorasick(patterns)

    expected = ["hello", "world", "hello"]

    # find_matches_as_indexes()
    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected

    # find_matches_as_strings()
    string_matches = ac.find_matches_as_strings(haystack)
    assert [string for (string, _) in string_matches] == expected
    assert [start for (_, start) in string_matches] == [
        start for (_, start, _) in index_matches
    ]


def test_matchkind():
    """
    Different matchkinds give different results.

    The default, MATCHKIND_STANDARD finds overlapping matches.

    MATCHKIND_LEFTMOST finds the leftmost match if there are overlapping
    matches, choosing the earlier provided pattern.

    MATCHKIND_LEFTMOST finds the leftmost match if there are overlapping
    matches, picking the longest one if there are multiple ones.
    """
    haystack = "This is the winter of my discontent"
    patterns = ["content", "disco", "disc", "discontent", "winter"]

    def get_strings(ac):
        result = ac.find_matches_as_strings(haystack)
        for string, start in result:
            assert haystack[start : start + len(string)] == string
        return [string for (string, _) in result]

    # Bad matchkind:
    with pytest.raises(ValueError):
        AhoCorasick(patterns, matchkind="Lalalala")

    # Default is MATCHKIND_STANDARD:
    assert get_strings(AhoCorasick(patterns)) == [
        "winter",
        "disc",
    ]

    # Explicit MATCHKIND_STANDARD:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)) == [
        "winter",
        "disc",
    ]

    # MATCHKIND_LEFTMOST_FIRST:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)) == [
        "winter",
        "disco",
    ]

    # MATCHKIND_LEFTMOST_LONGEST:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)) == [
        "winter",
        "discontent",
    ]


def test_overlapping():
    """
    It's possible to get overlapping matches, but only with MATCHKIND_STANDARD.
    """
    haystack = "This is the winter of my discontent"
    patterns = ["content", "disco", "disc", "discontent", "winter"]

    def get_strings(ac):
        assert ac.find_matches_as_strings(haystack) == ac.find_matches_as_strings(
            haystack, overlapping=False
        )
        assert ac.find_matches_as_indexes(haystack) == ac.find_matches_as_indexes(
            haystack, overlapping=False
        )
        result = ac.find_matches_as_strings(haystack, overlapping=True)
        for string, start in result:
            assert haystack[start : start + len(string)] == string
        result = [string for (string, _) in result]
        result_indexes = ac.find_matches_as_indexes(haystack, overlapping=True)
        assert [patterns[i] for (i, _, _) in result_indexes] == result
        assert [haystack[s:e] for (_, s, e) in result_indexes] == result
        return result

    def assert_no_overlapping(ac):
        with pytest.raises(ValueError):
            ac.find_matches_as_strings(haystack, overlapping=True)
        with pytest.raises(ValueError):
            ac.find_matches_as_indexes(haystack, overlapping=True)

    # Default is MATCHKIND_STANDARD:
    expected = [
        "winter",
        "disc",
        "disco",
        "discontent",
        "content",
    ]
    assert get_strings(AhoCorasick(patterns)) == expected

    # Explicit MATCHKIND_STANDARD:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)) == expected

    # Other matchkinds don't support overlapping.
    assert_no_overlapping(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST))
    assert_no_overlapping(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST))

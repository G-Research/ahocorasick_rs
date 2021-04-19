"""Tests for ahocorasick_rs."""

from ahocorasick_rs import AhoCorasick


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

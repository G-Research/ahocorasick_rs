"""Tests for ahocorasick_rs."""

from __future__ import annotations

from typing import Optional

import pytest

from hypothesis import strategies as st
from hypothesis import given, assume

from ahocorasick_rs import (
    AhoCorasick,
    MATCHKIND_STANDARD,
    MATCHKIND_LEFTMOST_FIRST,
    MATCHKIND_LEFTMOST_LONGEST,
    MatchKind,
    Implementation,
)


@pytest.mark.parametrize("store_patterns", [True, False, None])
@pytest.mark.parametrize(
    "implementation",
    [
        None,
        Implementation.NoncontiguousNFA,
        Implementation.ContiguousNFA,
        Implementation.DFA,
    ],
)
def test_basic_matching(
    store_patterns: Optional[bool], implementation: Optional[Implementation]
) -> None:
    """
    find_matches_as_indexes() and find_matches_as_strings() return matching
    patterns in the given string.
    """
    haystack = "hello, world, hello again"
    patterns = ["hello", "world"]
    if store_patterns is None:
        ac = AhoCorasick(patterns)
    else:
        ac = AhoCorasick(
            patterns, store_patterns=store_patterns, implementation=implementation
        )

    expected = ["hello", "world", "hello"]

    # find_matches_as_indexes()
    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected

    # find_matches_as_strings()
    assert ac.find_matches_as_strings(haystack) == expected


@pytest.mark.parametrize("store_patterns", [True, False, None])
def test_iterator_of_patterns(store_patterns: Optional[bool]) -> None:
    """
    It's possible to construct ``AhoCorasick()`` with an iterator.
    """
    haystack = "hello, world, hello again"
    patterns = iter(["hello", "world"])
    if store_patterns is None:
        ac = AhoCorasick(patterns)
    else:
        ac = AhoCorasick(patterns, store_patterns=store_patterns)

    expected = ["hello", "world", "hello"]
    assert ac.find_matches_as_strings(haystack) == expected


def test_bad_iterators() -> None:
    """
    When constructed with a bad iterator, the underlying Python error is raised.
    """
    with pytest.raises(TypeError):
        AhoCorasick(None)  # type: ignore

    with pytest.raises(TypeError):
        AhoCorasick(["x", 12])  # type: ignore


@given(
    st.lists(st.text(min_size=3), min_size=1, max_size=30_000),
    st.sampled_from([True, False, None]),
)
def test_construction_extensive(
    patterns: list[str], store_patterns: Optional[bool]
) -> None:
    """
    Exercise the construction code paths, ensuring we end up using all
    patterns.
    """
    patterns = [f"@{p}@" for p in patterns]
    ac = AhoCorasick(patterns, store_patterns=store_patterns)
    for p in patterns:
        assert ac.find_matches_as_strings(p) == [p]


@pytest.mark.parametrize("store_patterns", [True, False, None])
@pytest.mark.parametrize(
    "implementation",
    [
        None,
        Implementation.NoncontiguousNFA,
        Implementation.ContiguousNFA,
        Implementation.DFA,
    ],
)
def test_unicode(
    store_patterns: Optional[bool], implementation: Optional[Implementation]
) -> None:
    """
    Non-ASCII unicode patterns still give correct results for
    find_matches_as_indexes() and find_matches_as_strings().
    """
    haystack = "hello, world ☃fishá l🤦l"
    patterns = ["d ☃f", "há", "l🤦l"]
    if store_patterns is None:
        ac = AhoCorasick(patterns)
    else:
        ac = AhoCorasick(
            patterns, store_patterns=store_patterns, implementation=implementation
        )
    index_matches = ac.find_matches_as_indexes(haystack)
    expected = ["d ☃f", "há", "l🤦l"]
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected
    assert ac.find_matches_as_strings(haystack) == expected


@given(st.text(), st.text(min_size=1), st.text(), st.sampled_from([True, False, None]))
def test_unicode_extensive(
    prefix: str, pattern: str, suffix: str, store_patterns: Optional[bool]
) -> None:
    """
    Non-ASCII unicode patterns still give correct results for
    find_matches_as_indexes(), with property-testing.
    """
    assume(pattern not in prefix)
    assume(pattern not in suffix)
    haystack = prefix + pattern + suffix
    if store_patterns is None:
        ac = AhoCorasick([pattern])
    else:
        ac = AhoCorasick([pattern], store_patterns=store_patterns)

    index_matches = ac.find_matches_as_indexes(haystack)
    expected = [pattern]
    assert [i for (i, _, _) in index_matches] == [0]
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected
    assert ac.find_matches_as_strings(haystack) == [pattern]


@pytest.mark.parametrize("bad_patterns", [[""], ["", "xx"], ["xx", ""]])
@pytest.mark.parametrize("store_patterns", [True, False])
def test_empty_patterns_are_not_legal(
    bad_patterns: list[str], store_patterns: bool
) -> None:
    """
    Passing in an empty pattern suggests a bug in user code, and the outputs
    are bad when you do have that, so raise an error.
    """
    with pytest.raises(ValueError) as e:
        AhoCorasick(bad_patterns, store_patterns=store_patterns)
    assert "You passed in an empty string as a pattern" in str(e.value)


@given(st.text(min_size=1), st.text(), st.sampled_from([True, False, None]))
def test_unicode_totally_random(
    pattern: str, haystack: str, store_patterns: Optional[bool]
) -> None:
    """
    Catch more edge cases of patterns and haystacks.
    """
    if store_patterns is None:
        ac = AhoCorasick([pattern])
    else:
        ac = AhoCorasick([pattern], store_patterns=store_patterns)

    index_matches = ac.find_matches_as_indexes(haystack)
    string_matches = ac.find_matches_as_strings(haystack)

    expected_index = haystack.find(pattern)
    if expected_index == -1:
        assert index_matches == []
        assert string_matches == []
    else:
        assert index_matches[0][1] == expected_index
        assert [haystack[s:e] for (_, s, e) in index_matches][0] == pattern
        assert string_matches[0] == pattern


def test_matchkind() -> None:
    """
    Different matchkinds give different results.

    The default, MATCHKIND_STANDARD finds overlapping matches.

    MATCHKIND_LEFTMOST_FIRST finds the leftmost match if there are overlapping
    matches, choosing the earlier provided pattern.

    MATCHKIND_LEFTMOST_LONGEST finds the leftmost match if there are overlapping
    matches, picking the longest one if there are multiple ones.
    """
    haystack = "This is the winter of my discontent"
    patterns = ["content", "disco", "disc", "discontent", "winter"]

    def get_strings(ac: AhoCorasick) -> list[str]:
        return ac.find_matches_as_strings(haystack)

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
    assert get_strings(AhoCorasick(patterns, matchkind=MatchKind.Standard)) == [
        "winter",
        "disc",
    ]

    # MATCHKIND_LEFTMOST_FIRST:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)) == [
        "winter",
        "disco",
    ]
    assert get_strings(AhoCorasick(patterns, matchkind=MatchKind.LeftmostFirst)) == [
        "winter",
        "disco",
    ]

    # MATCHKIND_LEFTMOST_LONGEST:
    assert get_strings(AhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)) == [
        "winter",
        "discontent",
    ]
    assert get_strings(AhoCorasick(patterns, matchkind=MatchKind.LeftmostLongest)) == [
        "winter",
        "discontent",
    ]


def test_overlapping() -> None:
    """
    It's possible to get overlapping matches, but only with MATCHKIND_STANDARD.
    """
    haystack = "This is the winter of my discontent"
    patterns = ["content", "disco", "disc", "discontent", "winter"]

    def get_strings(ac: AhoCorasick) -> list[str]:
        assert ac.find_matches_as_strings(haystack) == ac.find_matches_as_strings(
            haystack, overlapping=False
        )
        assert ac.find_matches_as_indexes(haystack) == ac.find_matches_as_indexes(
            haystack, overlapping=False
        )
        result = ac.find_matches_as_strings(haystack, overlapping=True)
        result_indexes = ac.find_matches_as_indexes(haystack, overlapping=True)
        assert [patterns[i] for (i, _, _) in result_indexes] == result
        assert [haystack[s:e] for (_, s, e) in result_indexes] == result
        return result

    def assert_no_overlapping(ac: AhoCorasick) -> None:
        with pytest.raises(ValueError):
            ac.find_matches_as_strings(haystack, overlapping=True)
        with pytest.raises(ValueError):
            ac.find_matches_as_indexes(haystack, overlapping=True)

    # Default is MatchKind.Standard:
    expected = [
        "winter",
        "disc",
        "disco",
        "discontent",
        "content",
    ]
    assert get_strings(AhoCorasick(patterns)) == expected

    # Explicit MATCHKIND_STANDARD:
    assert get_strings(AhoCorasick(patterns, matchkind=MatchKind.Standard)) == expected

    # Other matchkinds don't support overlapping.
    assert_no_overlapping(AhoCorasick(patterns, matchkind=MatchKind.LeftmostFirst))
    assert_no_overlapping(AhoCorasick(patterns, matchkind=MatchKind.LeftmostLongest))

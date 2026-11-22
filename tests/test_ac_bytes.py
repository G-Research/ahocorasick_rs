"""Tests for ahocorasick_rs's bytes support."""

from __future__ import annotations

import gc
import sys
import weakref
from typing import Optional

import pytest

from hypothesis import strategies as st
from hypothesis import given

from ahocorasick_rs import (
    BytesAhoCorasick,
    MATCHKIND_STANDARD,
    MATCHKIND_LEFTMOST_FIRST,
    MATCHKIND_LEFTMOST_LONGEST,
    MatchKind,
    Implementation,
)

requires_python_buffer = pytest.mark.skipif(
    sys.version_info < (3, 12), reason="requires the Python-level buffer protocol"
)


@pytest.mark.parametrize(
    "implementation",
    [
        None,
        Implementation.NoncontiguousNFA,
        Implementation.ContiguousNFA,
        Implementation.DFA,
    ],
)
def test_basic_matching(implementation: Optional[Implementation]) -> None:
    """
    find_matches_as_indexes() returns matching patterns in the given byte string.
    """
    haystack = b"hello, world, hello again"
    patterns = [b"hello", b"world"]
    ac = BytesAhoCorasick(patterns, implementation=implementation)

    expected = [b"hello", b"world", b"hello"]

    # find_matches_as_indexes()
    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected


@pytest.mark.parametrize(
    "implementation",
    [
        None,
        Implementation.NoncontiguousNFA,
        Implementation.ContiguousNFA,
        Implementation.DFA,
    ],
)
def test_different_byte_objects_matching(
    implementation: Optional[Implementation],
) -> None:
    """
    find_matches_as_indexes() returns matching patterns in the given byte string.
    """
    haystack = b"hello, world, hello again"
    patterns = [memoryview(b"hello"), bytearray(b"world")]
    ac = BytesAhoCorasick(patterns, implementation=implementation)  # type: ignore

    expected = [b"hello", b"world", b"hello"]

    # find_matches_as_indexes()
    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected


@pytest.mark.parametrize(
    "implementation",
    [
        None,
        Implementation.NoncontiguousNFA,
        Implementation.ContiguousNFA,
        Implementation.DFA,
    ],
)
@pytest.mark.parametrize("haystack_type", [bytes, bytearray, memoryview])
def test_different_byte_haystacks_matching(
    implementation: Optional[Implementation],
    haystack_type: type[bytes | bytearray | memoryview],
) -> None:
    """
    find_matches_as_indexes() returns matching patterns in the given byte string.
    """
    haystack = haystack_type(b"hello, world, hello again")
    patterns = [b"hello", b"world"]
    ac = BytesAhoCorasick(patterns, implementation=implementation)

    expected = [b"hello", b"world", b"hello"]

    # find_matches_as_indexes()
    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected


@pytest.mark.parametrize("haystack_type", [bytes, bytearray])
def test_find_matches_as_indexes_iter(haystack_type: type[bytes | bytearray]) -> None:
    haystack = haystack_type(b"xabc!abc")
    ac = BytesAhoCorasick([b"ab", b"abc", b"bc", b"!"])

    expected = [(3, 0), (4, 1), (4, 2), (5, 3), (7, 0), (8, 1), (8, 2)]
    assert expected == [
        (end, idx)
        for idx, _start, end in ac.find_matches_as_indexes(haystack, overlapping=True)
    ]
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=True)
    assert iter(matches) is matches
    assert next(matches) == expected[0]
    assert list(matches) == expected[1:]
    assert list(matches) == []
    non_overlapping = [
        (end, idx) for idx, _start, end in ac.find_matches_as_indexes(haystack)
    ]
    assert list(ac.find_matches_as_indexes_iter(haystack)) == non_overlapping


def test_match_iterator_preserves_overlaps_across_refills() -> None:
    haystack = b"\xff" + b"ab" * 136
    ac = BytesAhoCorasick([b"ab", b"b"])
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=True)
    expected = [
        (end, index) for end in range(3, len(haystack) + 1, 2) for index in (0, 1)
    ]

    assert next(matches) == (3, 0)
    assert list(matches) == expected[1:]
    assert next(matches, None) is None


@pytest.mark.parametrize("overlapping", [False, True])
def test_find_matches_as_indexes_iter_releases_mutable_buffer(
    overlapping: bool,
) -> None:
    haystack = bytearray(b"xabc!abc" * 3)
    ac = BytesAhoCorasick([b"ab", b"abc", b"bc", b"!"])
    expected = [
        (end, idx)
        for idx, _start, end in ac.find_matches_as_indexes(
            haystack, overlapping=overlapping
        )
    ]
    assert len(expected) > 3

    # The buffer stays locked until the iterator is exhausted...
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=overlapping)
    assert next(matches) == expected[0]
    with pytest.raises(BufferError):
        haystack.append(0)
    assert list(matches) == expected[1:]

    # ...and is released then,
    haystack.append(0)

    # or when the iterator is dropped early.
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=overlapping)
    assert next(matches) == expected[0]
    del matches
    haystack.append(0)


def test_find_matches_as_indexes_iter_validates_overlapping() -> None:
    ac = BytesAhoCorasick([b"a"], matchkind=MatchKind.LeftmostFirst)
    with pytest.raises(ValueError):
        ac.find_matches_as_indexes_iter(b"", overlapping=True)


@requires_python_buffer
def test_find_matches_as_indexes_iter_keeps_exported_buffer_stable() -> None:
    class ChangingExporter(bytes):
        exports = 0

        def __buffer__(self, flags: int) -> memoryview:
            self.exports += 1
            if self.exports == 1:
                return memoryview(b"xxxaaa")
            return memoryview(b"x")

    ac = BytesAhoCorasick([b"a"])
    haystack = ChangingExporter(b"x")
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=True)
    assert list(matches) == [(4, 0), (5, 0), (6, 0)]


@pytest.mark.parametrize(
    "custom_exporter",
    [False, pytest.param(True, marks=requires_python_buffer)],
    ids=["bytearray", "custom-exporter"],
)
def test_find_matches_as_indexes_iter_collects_exporter_cycles(
    custom_exporter: bool,
) -> None:
    class BytearrayExporter(bytearray):
        iterator: object

    class CustomExporter:
        iterator: object

        def __buffer__(self, flags: int) -> memoryview:
            return memoryview(b"a")

    ac = BytesAhoCorasick([b"a"])
    haystack = CustomExporter() if custom_exporter else BytearrayExporter(b"a")
    haystack_ref = weakref.ref(haystack)
    matches = ac.find_matches_as_indexes_iter(haystack)
    haystack.iterator = matches
    del haystack, matches

    gc.collect()
    assert haystack_ref() is None


def test_iterator_of_patterns() -> None:
    """
    It's possible to construct ``BytesAhoCorasick()`` with an iterator.
    """
    haystack = b"hello, world, hello again"
    patterns = [b"hello", b"world"]
    ac = BytesAhoCorasick(iter(patterns))

    expected = [b"hello", b"world", b"hello"]

    index_matches = ac.find_matches_as_indexes(haystack)
    assert [patterns[i] for (i, _, _) in index_matches] == expected
    assert [haystack[s:e] for (_, s, e) in index_matches] == expected


def test_bad_iterators() -> None:
    """
    When constructed with a bad iterator, the underlying Python error is raised.
    """
    with pytest.raises(TypeError):
        BytesAhoCorasick(None)  # type: ignore

    with pytest.raises(TypeError):
        BytesAhoCorasick([b"x", 12])  # type: ignore[list-item]

    # str doesn't implement the buffer API and can't be converted to bytes
    with pytest.raises(TypeError):
        BytesAhoCorasick([b"x", "y"])  # type: ignore[list-item]


@given(
    st.lists(st.binary(min_size=3), min_size=1, max_size=30_000),
)
def test_construction_extensive(patterns: list[bytes]) -> None:
    """
    Exercise the construction code paths, ensuring we end up using all
    patterns.
    """
    patterns = [b"%b_%i_" % (p, i) for (i, p) in enumerate(patterns)]
    ac = BytesAhoCorasick(patterns)
    for haystack in patterns:
        assert [
            haystack[s:e] for (_, s, e) in ac.find_matches_as_indexes(haystack)
        ] == [haystack]


@given(st.binary(), st.binary(min_size=1), st.binary())
def test_random_bytes_extensive(prefix: bytes, pattern: bytes, suffix: bytes) -> None:
    """
    Random bytes patterns still give correct results for
    find_matches_as_indexes(), with property-testing.
    """
    haystack = prefix + pattern + suffix
    ac = BytesAhoCorasick([pattern])

    index_matches = ac.find_matches_as_indexes(haystack)
    assert {i for (i, _, _) in index_matches} == {0}
    # Occasionally might get overlap between haystack and prefix/suffix...
    assert {haystack[s:e] for (_, s, e) in index_matches} == {pattern}


@pytest.mark.parametrize("bad_patterns", [[b""], [b"", b"xx"], [b"xx", b""]])
def test_empty_patterns_are_not_legal(bad_patterns: list[bytes]) -> None:
    """
    Passing in an empty pattern suggests a bug in user code, and the outputs
    are bad when you do have that, so raise an error.
    """
    with pytest.raises(ValueError) as e:
        BytesAhoCorasick(bad_patterns)
    assert "You passed in an empty pattern" in str(e.value)


@given(st.binary(min_size=1), st.binary())
def test_bytes_totally_random(pattern: bytes, haystack: bytes) -> None:
    """
    Catch more edge cases of patterns and haystacks.
    """
    ac = BytesAhoCorasick([pattern])

    index_matches = ac.find_matches_as_indexes(haystack)

    expected_index = haystack.find(pattern)
    if expected_index == -1:
        assert index_matches == []
    else:
        assert index_matches[0][1] == expected_index
        assert [haystack[s:e] for (_, s, e) in index_matches][0] == pattern


def test_matchkind() -> None:
    """
    Different matchkinds give different results.

    The default, MATCHKIND_STANDARD finds overlapping matches.

    MATCHKIND_LEFTMOST_FIRST finds the leftmost match if there are overlapping
    matches, choosing the earlier provided pattern.

    MATCHKIND_LEFTMOST_LONGEST finds the leftmost match if there are overlapping
    matches, picking the longest one if there are multiple ones.
    """
    haystack = b"This is the winter of my discontent"
    patterns = [b"content", b"disco", b"disc", b"discontent", b"winter"]

    def get_strings(ac: BytesAhoCorasick) -> list[bytes]:
        return [haystack[s:e] for (_, s, e) in ac.find_matches_as_indexes(haystack)]

    # Default is MATCHKIND_STANDARD:
    assert get_strings(BytesAhoCorasick(patterns)) == [
        b"winter",
        b"disc",
    ]

    # Explicit MATCHKIND_STANDARD:
    assert get_strings(BytesAhoCorasick(patterns, matchkind=MATCHKIND_STANDARD)) == [
        b"winter",
        b"disc",
    ]
    assert get_strings(BytesAhoCorasick(patterns, matchkind=MatchKind.Standard)) == [
        b"winter",
        b"disc",
    ]

    # MATCHKIND_LEFTMOST_FIRST:
    assert get_strings(
        BytesAhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_FIRST)
    ) == [
        b"winter",
        b"disco",
    ]
    assert get_strings(
        BytesAhoCorasick(patterns, matchkind=MatchKind.LeftmostFirst)
    ) == [
        b"winter",
        b"disco",
    ]

    # MATCHKIND_LEFTMOST_LONGEST:
    assert get_strings(
        BytesAhoCorasick(patterns, matchkind=MATCHKIND_LEFTMOST_LONGEST)
    ) == [
        b"winter",
        b"discontent",
    ]
    assert get_strings(
        BytesAhoCorasick(patterns, matchkind=MatchKind.LeftmostLongest)
    ) == [
        b"winter",
        b"discontent",
    ]


def test_overlapping() -> None:
    """
    It's possible to get overlapping matches, but only with MATCHKIND_STANDARD.
    """
    haystack = b"This is the winter of my discontent"
    patterns = [b"content", b"disco", b"disc", b"discontent", b"winter"]

    def get_strings(ac: BytesAhoCorasick) -> list[bytes]:
        assert ac.find_matches_as_indexes(haystack) == ac.find_matches_as_indexes(
            haystack, overlapping=False
        )
        return [
            haystack[s:e]
            for (_, s, e) in ac.find_matches_as_indexes(haystack, overlapping=True)
        ]

    def assert_no_overlapping(ac: BytesAhoCorasick) -> None:
        with pytest.raises(ValueError):
            ac.find_matches_as_indexes(haystack, overlapping=True)

    # Default is MatchKind.Standard:
    expected = [
        b"winter",
        b"disc",
        b"disco",
        b"discontent",
        b"content",
    ]
    assert get_strings(BytesAhoCorasick(patterns)) == expected

    # Explicit MATCHKIND_STANDARD:
    assert (
        get_strings(BytesAhoCorasick(patterns, matchkind=MatchKind.Standard))
        == expected
    )

    # Other matchkinds don't support overlapping.
    assert_no_overlapping(BytesAhoCorasick(patterns, matchkind=MatchKind.LeftmostFirst))
    assert_no_overlapping(
        BytesAhoCorasick(patterns, matchkind=MatchKind.LeftmostLongest)
    )

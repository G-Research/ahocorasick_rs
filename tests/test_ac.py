"""Tests for ahocorasick_rs."""

from __future__ import annotations

import gc
import sys
import threading
import weakref

from functools import partial
from typing import Callable, Optional

import pytest

from hypothesis import strategies as st
from hypothesis import given

from ahocorasick_rs import (
    AhoCorasick,
    BytesAhoCorasick,
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
    patterns = [f"{p}_{i}_" for (i, p) in enumerate(patterns)]
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
    haystack = prefix + pattern + suffix
    if store_patterns is None:
        ac = AhoCorasick([pattern])
    else:
        ac = AhoCorasick([pattern], store_patterns=store_patterns)

    index_matches = ac.find_matches_as_indexes(haystack)
    expected = {pattern}
    assert {i for (i, _, _) in index_matches} == {0}
    # Occasionally might get an overlap between haystack and preffix/suffix...
    assert {haystack[s:e] for (_, s, e) in index_matches} == expected
    assert set(ac.find_matches_as_strings(haystack)) == expected


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
        with pytest.raises(ValueError):
            ac.find_matches_as_indexes_iter(haystack, overlapping=True)

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


def test_find_matches_as_indexes_iter() -> None:
    haystack = "🙂 winter disc discontent"
    patterns = ["disc", "disco", "discontent", "winter"]
    ac = AhoCorasick(patterns)

    expected = [
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


@pytest.mark.parametrize(
    "haystack",
    [
        "winter disc discontent winter",
        "wintér disc discontent winter",
        "☃ winter 🙂 disc é discontent 🙂 winter",
        "é🙂☃winter",
        "disc🙂",
    ],
    ids=[
        "ascii",
        "latin1",
        "multibyte-before-matches",
        "match-at-end",
        "match-at-start",
    ],
)
@pytest.mark.parametrize(
    "matchkind, overlapping",
    [
        (MatchKind.Standard, False),
        (MatchKind.Standard, True),
        (MatchKind.LeftmostFirst, False),
        (MatchKind.LeftmostLongest, False),
    ],
)
def test_indexes_are_code_points_for_ascii_and_non_ascii(
    haystack: str, matchkind: MatchKind, overlapping: bool
) -> None:
    """
    Indexes are code point offsets, whether or not the haystack is ASCII
    (byte offsets only match code point offsets for ASCII).
    """
    # "inter" shares its end with "winter", exercising pending overlapping matches.
    patterns = ["disc", "disco", "discontent", "winter", "inter", "🙂 winter"]
    ac = AhoCorasick(patterns, matchkind=matchkind)
    bytes_ac = BytesAhoCorasick([p.encode() for p in patterns], matchkind=matchkind)
    encoded = haystack.encode()

    def to_code_point(byte_offset: int) -> int:
        return len(encoded[:byte_offset].decode())

    # The bytes API returns byte offsets, which we convert independently.
    expected = [
        (pattern, to_code_point(start), to_code_point(end))
        for pattern, start, end in bytes_ac.find_matches_as_indexes(
            encoded, overlapping=overlapping
        )
    ]
    assert expected

    matches = ac.find_matches_as_indexes(haystack, overlapping=overlapping)
    assert matches == expected
    assert [haystack[start:end] for _, start, end in matches] == [
        patterns[pattern] for pattern, _, _ in matches
    ]
    assert list(ac.find_matches_as_indexes_iter(haystack, overlapping=overlapping)) == [
        (end, pattern) for pattern, _, end in expected
    ]


class _ByteLengthStr(str):
    """A ``str`` subclass whose ``__len__`` is the UTF-8 byte length."""

    def __len__(self) -> int:
        return len(self.encode())


class _RaisingLenStr(str):
    """A ``str`` subclass whose ``__len__`` raises."""

    def __len__(self) -> int:
        raise RuntimeError("__len__ must not be called")


@pytest.mark.parametrize("str_type", [_ByteLengthStr, _RaisingLenStr])
def test_indexes_ignore_str_subclass_len(str_type: type[str]) -> None:
    """
    Indexes are true code point offsets, even if a ``str`` subclass overrides
    ``__len__``.
    """
    ac = AhoCorasick(["winter"])
    haystack = str_type("é winter")
    assert ac.find_matches_as_indexes(haystack) == [(0, 2, 8)]
    assert list(ac.find_matches_as_indexes_iter(haystack)) == [(8, 0)]


def test_find_matches_as_indexes_iter_dense_unicode_overlaps() -> None:
    unit = "é🙂e"
    width = 512
    haystack = "🧪a" + unit * 4096 + "終"
    ac = AhoCorasick([unit * width, unit], store_patterns=False)
    matches = ac.find_matches_as_indexes_iter(haystack, overlapping=True)

    for occurrence in range(1, 4097):
        end = 2 + 3 * occurrence
        if occurrence >= width:
            assert next(matches) == (end, 0)
        assert next(matches) == (end, 1)
    assert next(matches, None) is None


def test_match_iterators_keep_independent_unicode_refills() -> None:
    ac = AhoCorasick(["🙂é", "é"])
    left = ac.find_matches_as_indexes_iter("🙂é" * 70, overlapping=True)
    right = ac.find_matches_as_indexes_iter("x🙂é" * 70, overlapping=True)

    for occurrence in range(1, 71):
        for pattern in (0, 1):
            assert next(left) == (2 * occurrence, pattern)
            assert next(right) == (3 * occurrence, pattern)
    assert next(left, None) is None
    assert next(right, None) is None


@pytest.mark.parametrize(
    "make_matcher",
    [partial(AhoCorasick, ["a"]), partial(BytesAhoCorasick, [b"a"])],
    ids=["str", "bytes"],
)
def test_batch_size_is_not_a_constructor_option(
    make_matcher: Callable[..., object],
) -> None:
    with pytest.raises(TypeError, match="iter_batch_size"):
        make_matcher(iter_batch_size=1)


def test_find_matches_as_indexes_iter_participates_in_garbage_collection() -> None:
    class Haystack(str):
        iterator: object

    ac = AhoCorasick(["a"])
    haystack = Haystack("a")
    haystack_ref = weakref.ref(haystack)
    matches = ac.find_matches_as_indexes_iter(haystack)
    haystack.iterator = matches
    del haystack, matches

    gc.collect()
    assert haystack_ref() is None


def test_match_iterator_holds_haystack_until_it_reports_exhaustion() -> None:
    class Haystack(str):
        pass

    haystack = Haystack("aaa")
    haystack_ref = weakref.ref(haystack)
    matches = AhoCorasick(["a"]).find_matches_as_indexes_iter(haystack)
    del haystack

    assert next(matches) == (1, 0)
    assert next(matches) == (2, 0)
    assert haystack_ref() is not None
    assert next(matches) == (3, 0)
    assert haystack_ref() is not None
    assert next(matches, None) is None
    assert haystack_ref() is None


@pytest.mark.skipif(
    not getattr(sys, "_is_gil_enabled", lambda: True)(), reason="requires the GIL"
)
@pytest.mark.parametrize("as_bytes", [False, True], ids=["str", "bytes"])
@pytest.mark.parametrize(
    "after_first",
    [pytest.param(False, id="first_scan"), pytest.param(True, id="refill")],
)
def test_find_matches_as_indexes_iter_releases_gil(
    as_bytes: bool, after_first: bool
) -> None:
    # The overlapping search visits repeated partial matches through the
    # automaton. Allow several searches for the OS to schedule the worker.
    haystack = ("ab" if after_first else "") + "a" * 8_000_000
    if as_bytes:
        ac_bytes = BytesAhoCorasick([b"ab", b"b"])
        haystack_bytes = haystack.encode()
        searches = [
            ac_bytes.find_matches_as_indexes_iter(haystack_bytes, overlapping=True)
            for _ in range(4)
        ]
    else:
        ac = AhoCorasick(["ab", "b"])
        searches = [
            ac.find_matches_as_indexes_iter(haystack, overlapping=True)
            for _ in range(4)
        ]

    if after_first:
        for matches in searches:
            assert next(matches) == (2, 0)
    expected = (2, 1) if after_first else None

    ready = threading.Event()
    searching = threading.Event()
    progressed = threading.Event()

    def make_progress() -> None:
        ready.set()
        searching.wait()
        progressed.set()

    worker = threading.Thread(target=make_progress)
    worker.start()
    old_interval = sys.getswitchinterval()
    # Suppress normal interpreter thread switches between the search calls.
    sys.setswitchinterval(60.0)
    try:
        assert ready.wait(timeout=5)
        searching.set()
        assert not progressed.is_set()
        for matches in searches:
            assert next(matches, None) == expected
            if progressed.is_set():
                break
        assert progressed.is_set()
    finally:
        sys.setswitchinterval(old_interval)
        searching.set()
        worker.join()

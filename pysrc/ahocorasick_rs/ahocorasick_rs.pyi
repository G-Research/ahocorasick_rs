from __future__ import annotations

from typing import Optional, Iterable
from collections.abc import Buffer

class Implementation:
    NoncontiguousNFA: Implementation
    ContiguousNFA: Implementation
    DFA: Implementation

class MatchKind:
    Standard: MatchKind
    LeftmostFirst: MatchKind
    LeftmostLongest: MatchKind

class AhoCorasick:
    def __init__(
        self,
        patterns: Iterable[str],
        matchkind: MatchKind = MatchKind.Standard,
        store_patterns: Optional[bool] = None,
        implementation: Optional[Implementation] = None,
    ) -> None: ...
    def find_matches_as_indexes(
        self, haystack: str, overlapping: bool = False
    ) -> list[tuple[int, int, int]]: ...
    def find_matches_as_strings(
        self, haystack: str, overlapping: bool = False
    ) -> list[str]: ...

class BytesAhoCorasick:
    def __init__(
        self,
        patterns: Iterable[Buffer],
        matchkind: MatchKind = MatchKind.Standard,
        implementation: Optional[Implementation] = None,
    ) -> None: ...
    def find_matches_as_indexes(
        self, haystack: Buffer, overlapping: bool = False
    ) -> list[tuple[int, int, int]]: ...

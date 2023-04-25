from typing import Optional

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
        patterns: list[str],
        matchkind: MatchKind = MatchKind.Standard,
        store_patterns: Optional[bool] = None,
        implementation: Optional[Implementation] = Implementation.DFA,
    ) -> None: ...
    def find_matches_as_indexes(
        self, haystack: str, overlapping: bool = False
    ) -> list[tuple[int, int, int]]: ...
    def find_matches_as_strings(
        self, haystack: str, overlapping: bool = False
    ) -> list[str]: ...

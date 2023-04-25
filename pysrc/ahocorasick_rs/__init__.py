# Expose the Rust code:
from .ahocorasick_rs import (
    AhoCorasick,
    MatchKind,
    Implementation,
)

# Backwards compatibility:
MATCHKIND_STANDARD = MatchKind.Standard
MATCHKIND_LEFTMOST_FIRST = MatchKind.LeftmostFirst
MATCHKIND_LEFTMOST_LONGEST = MatchKind.LeftmostLongest

__all__ = [
    "AhoCorasick",
    "MatchKind",
    "Implementation",
    # Deprecated:
    "MATCHKIND_STANDARD",
    "MATCHKIND_LEFTMOST_FIRST",
    "MATCHKIND_LEFTMOST_LONGEST",
]

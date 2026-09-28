"""Policy values for the source-shape checker.

Keep this module declarative. Traversal, reporting, and command-line behavior
belong to ``scripts/check/source_shape.py``.
"""

WARN_LINES = 600
FAIL_LINES = 1000

SOURCE_SUFFIXES = frozenset({".rs"})

EXCLUDED_PARTS = frozenset(
    {
        ".git",
        "target",
        "vendor",
        "vendors",
        "generated",
        "migrations",
        "fixtures",
        "tests",
    }
)

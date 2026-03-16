"""Visibility types for capabilities."""

from enum import Enum


class Visibility(str, Enum):
    """Visibility level for skills, tools, and MCPs."""

    PUBLIC = "public"
    """Public: Available to all agents."""

    PRIVATE = "private"
    """Private: Must be explicitly loaded by agents."""

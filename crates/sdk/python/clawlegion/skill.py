"""Skill system for Python plugins."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from .visibility import Visibility


@dataclass
class SkillMetadata:
    """Metadata for a skill."""

    name: str
    """Unique skill name."""

    version: str
    """Skill version (semver)."""

    description: str
    """Skill description."""

    visibility: Visibility = Visibility.PUBLIC
    """Skill visibility."""

    tags: List[str] = field(default_factory=list)
    """Tags for categorization and retrieval."""

    author: Optional[str] = None
    """Skill author."""

    required_tools: List[str] = field(default_factory=list)
    """Required tool names."""

    required_mcps: List[str] = field(default_factory=list)
    """Required MCP names."""

    dependencies: List[str] = field(default_factory=list)
    """Dependencies on other skills."""


@dataclass
class SkillContext:
    """Context provided during skill execution."""

    agent_id: str
    """Agent ID executing this skill."""

    config: Dict[str, Any] = field(default_factory=dict)
    """Skill configuration."""

    state: Dict[str, Any] = field(default_factory=dict)
    """Shared state across skill invocations."""

    def get_config(self, key: str, default: Any = None) -> Any:
        """Get a configuration value."""
        return self.config.get(key, default)

    def get_state(self, key: str, default: Any = None) -> Any:
        """Get a state value."""
        return self.state.get(key, default)

    def set_state(self, key: str, value: Any) -> None:
        """Set a state value."""
        self.state[key] = value


@dataclass
class SkillInput:
    """Input for skill execution."""

    text: Optional[str] = None
    """Text input."""

    data: Dict[str, Any] = field(default_factory=dict)
    """Structured data."""

    attachments: List[str] = field(default_factory=list)
    """Attached file paths."""

    @classmethod
    def from_text(cls, text: str) -> "SkillInput":
        """Create input from text."""
        return cls(text=text)

    @classmethod
    def from_data(cls, data: Dict[str, Any]) -> "SkillInput":
        """Create input from data."""
        return cls(data=data)


@dataclass
class SkillOutput:
    """Output from skill execution."""

    text: Optional[str] = None
    """Text result."""

    data: Optional[Dict[str, Any]] = None
    """Structured result."""

    success: bool = True
    """Whether the skill completed successfully."""

    error: Optional[str] = None
    """Error message if failed."""

    follow_ups: List[Dict[str, Any]] = field(default_factory=list)
    """Follow-up actions requested."""

    @classmethod
    def success(cls, text: Optional[str] = None, data: Optional[Dict[str, Any]] = None) -> "SkillOutput":
        """Create a successful output."""
        return cls(text=text, data=data, success=True)

    @classmethod
    def error(cls, message: str) -> "SkillOutput":
        """Create an error output."""
        return cls(success=False, error=message)


class Skill(ABC):
    """Base class for all ClawLegion skills."""

    @abstractmethod
    def metadata(self) -> SkillMetadata:
        """Get skill metadata."""
        pass

    @abstractmethod
    async def execute(self, ctx: SkillContext, input: SkillInput) -> SkillOutput:
        """Execute the skill.

        Args:
            ctx: Skill context with agent info and configuration.
            input: Skill input data.

        Returns:
            SkillOutput: Result of skill execution.

        Raises:
            Exception: If execution fails.
        """
        pass

    async def init(self, ctx: SkillContext) -> None:
        """Initialize the skill.

        Called when the skill is loaded by an agent.

        Args:
            ctx: Skill context.
        """
        pass

    def system_prompt(self) -> Optional[str]:
        """Get the skill's system prompt for LLM-based skills.

        Returns:
            Optional[str]: System prompt or None if not applicable.
        """
        return None

    async def shutdown(self) -> None:
        """Shutdown the skill.

        Called when the skill is being unloaded.
        """
        pass

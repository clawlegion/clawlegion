"""Tool system for Python plugins."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
from .visibility import Visibility


@dataclass
class ToolMetadata:
    """Metadata for a tool."""

    name: str
    """Unique tool name."""

    version: str
    """Tool version (semver)."""

    description: str
    """Tool description."""

    visibility: Visibility = Visibility.PUBLIC
    """Tool visibility."""

    tags: List[str] = field(default_factory=list)
    """Tags for categorization and retrieval."""

    input_schema: Dict[str, Any] = field(default_factory=lambda: {"type": "object"})
    """Input JSON schema."""

    output_schema: Optional[Dict[str, Any]] = None
    """Output JSON schema."""

    requires_llm: bool = False
    """Whether this tool requires LLM."""


@dataclass
class ToolContext:
    """Context provided during tool execution."""

    agent_id: str
    """Agent ID executing this tool."""

    config: Dict[str, Any] = field(default_factory=dict)
    """Tool configuration."""

    timeout_ms: Optional[int] = None
    """Execution timeout in milliseconds."""

    def get_config(self, key: str, default: Any = None) -> Any:
        """Get a configuration value."""
        return self.config.get(key, default)


@dataclass
class ToolResult:
    """Result from tool execution."""

    success: bool = True
    """Success flag."""

    data: Optional[Any] = None
    """Result data."""

    error: Optional[str] = None
    """Error message if failed."""

    execution_time_ms: int = 0
    """Execution time in milliseconds."""

    @classmethod
    def success(cls, data: Any) -> "ToolResult":
        """Create a successful result."""
        return cls(data=data, success=True)

    @classmethod
    def error(cls, message: str, execution_time_ms: int = 0) -> "ToolResult":
        """Create an error result."""
        return cls(success=False, error=message, execution_time_ms=execution_time_ms)


class Tool(ABC):
    """Base class for all ClawLegion tools."""

    @abstractmethod
    def metadata(self) -> ToolMetadata:
        """Get tool metadata."""
        pass

    @abstractmethod
    async def execute(self, ctx: ToolContext, args: Dict[str, Any]) -> ToolResult:
        """Execute the tool.

        Args:
            ctx: Tool context with agent info and configuration.
            args: Tool arguments (must match input_schema).

        Returns:
            ToolResult: Result of tool execution.

        Raises:
            Exception: If execution fails.
        """
        pass

    def description(self) -> str:
        """Get tool description for LLM prompts.

        Returns:
            str: Tool description.
        """
        return self.metadata().description

    async def validate_args(self, args: Dict[str, Any]) -> bool:
        """Validate tool arguments against input schema.

        Args:
            args: Arguments to validate.

        Returns:
            bool: True if valid, False otherwise.
        """
        # Simple validation - can be extended with jsonschema
        return isinstance(args, dict)

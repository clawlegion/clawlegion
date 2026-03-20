"""Agent types for Python plugins."""

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


class AgentType(str, Enum):
    """Agent type enumeration."""

    REACT = "react"
    """ReAct pattern agent (reasoning + acting)."""

    FLOW = "flow"
    """Flow-based agent (predefined workflows)."""

    NORMAL = "normal"
    """Normal agent (rule-based, no LLM)."""

    CODEX = "codex"
    """Codex CLI-backed agent."""

    CLAUDE_CODE = "claude_code"
    """Claude Code CLI-backed agent."""

    OPEN_CODE = "open_code"
    """OpenCode CLI-backed agent."""

    CUSTOM = "custom"
    """Custom agent type (plugin-defined)."""


@dataclass
class AgentConfig:
    """Configuration for an agent."""

    id: str
    """Unique agent ID."""

    company_id: str
    """Company ID this agent belongs to."""

    name: str
    """Agent name."""

    role: str
    """Agent role (e.g., 'ceo', 'engineer')."""

    title: str
    """Agent title (e.g., '首席执行官')."""

    agent_type: AgentType = AgentType.REACT
    """Agent type."""

    icon: Optional[str] = None
    """Agent icon (emoji or URL)."""

    reports_to: Optional[str] = None
    """ID of the manager agent (None for CEO)."""

    capabilities: str = ""
    """Agent capabilities description."""

    skills: List[str] = field(default_factory=list)
    """Skills loaded by this agent."""

    adapter_type: str = "default"
    """Adapter type for running this agent."""

    adapter_config: Dict[str, Any] = field(default_factory=dict)
    """Adapter-specific configuration."""

    runtime_config: Dict[str, Any] = field(default_factory=dict)
    """Runtime configuration."""

    tags: List[str] = field(default_factory=list)
    """Agent tags for categorization."""

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "id": self.id,
            "company_id": self.company_id,
            "name": self.name,
            "role": self.role,
            "title": self.title,
            "agent_type": self.agent_type.value,
            "icon": self.icon,
            "reports_to": self.reports_to,
            "capabilities": self.capabilities,
            "skills": self.skills,
            "adapter_type": self.adapter_type,
            "adapter_config": self.adapter_config,
            "runtime_config": self.runtime_config,
            "tags": self.tags,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AgentConfig":
        """Create from dictionary."""
        return cls(
            id=data["id"],
            company_id=data["company_id"],
            name=data["name"],
            role=data["role"],
            title=data["title"],
            agent_type=AgentType(data.get("agent_type", "react")),
            icon=data.get("icon"),
            reports_to=data.get("reports_to"),
            capabilities=data.get("capabilities", ""),
            skills=data.get("skills", []),
            adapter_type=data.get("adapter_type", "default"),
            adapter_config=data.get("adapter_config", {}),
            runtime_config=data.get("runtime_config", {}),
            tags=data.get("tags", []),
        )

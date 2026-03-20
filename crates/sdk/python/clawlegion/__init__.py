"""
ClawLegion Python SDK

SDK for developing Python plugins for the ClawLegion Multi-Agent System.
"""

from .plugin import (
    Plugin,
    PluginContext,
    PluginManifest,
    PluginMetadata,
    build_manifest,
    scaffold_plugin_new,
)
from .skill import Skill, SkillContext, SkillMetadata, SkillInput, SkillOutput
from .tool import Tool, ToolContext, ToolMetadata, ToolResult
from .agent import AgentType, AgentConfig
from .visibility import Visibility

__version__ = "0.1.0b1.dev202603201322"
__all__ = [
    # Plugin
    "Plugin",
    "PluginContext",
    "PluginManifest",
    "PluginMetadata",
    "build_manifest",
    "scaffold_plugin_new",

    # Skill
    "Skill",
    "SkillContext",
    "SkillMetadata",
    "SkillInput",
    "SkillOutput",

    # Tool
    "Tool",
    "ToolContext",
    "ToolMetadata",
    "ToolResult",

    # Agent
    "AgentType",
    "AgentConfig",

    # Visibility
    "Visibility",
]

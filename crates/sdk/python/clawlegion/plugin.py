"""Plugin base classes and types."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, Optional, List
from pathlib import Path


@dataclass
class PluginMetadata:
    """Metadata for a plugin."""

    name: str
    """Unique plugin name."""

    version: str
    """Plugin version (semver)."""

    description: str
    """Plugin description."""

    author: Optional[str] = None
    """Plugin author."""

    core_version: Optional[str] = None
    """Required ClawLegion core version."""

    dependencies: List[str] = field(default_factory=list)
    """Plugin dependencies."""

    tags: List[str] = field(default_factory=list)
    """Plugin tags for categorization."""


@dataclass
class PluginContext:
    """Context provided to plugins during initialization."""

    config: Dict[str, Any]
    """Plugin configuration."""

    data_dir: Path
    """Plugin data directory."""

    config_dir: Path
    """Plugin config directory."""

    def get_config(self, key: str, default: Any = None) -> Any:
        """Get a configuration value."""
        return self.config.get(key, default)

    def get_config_typed(self, key: str, type_: type, default: Any = None) -> Any:
        """Get a typed configuration value."""
        value = self.config.get(key, default)
        if isinstance(value, type_):
            return value
        return default


@dataclass
class PluginManifest:
    """Manifest model used by the Python SDK."""

    id: str
    version: str
    api_version: str
    runtime: str
    entrypoint: str
    metadata: PluginMetadata
    capabilities: List[Dict[str, Any]] = field(default_factory=list)
    permissions: List[Dict[str, Any]] = field(default_factory=list)
    dependencies: List[Dict[str, Any]] = field(default_factory=list)
    compatible_host_versions: List[str] = field(default_factory=list)
    signature: Optional[Dict[str, Any]] = None
    healthcheck: Optional[Dict[str, Any]] = None
    config_schema: Optional[Dict[str, Any]] = None
    ui_metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        metadata = {
            "name": self.metadata.name,
            "version": self.metadata.version,
            "description": self.metadata.description,
            "author": self.metadata.author,
            "core_version": self.metadata.core_version,
            "dependencies": self.metadata.dependencies,
            "tags": self.metadata.tags,
        }
        return {
            "id": self.id,
            "version": self.version,
            "api_version": self.api_version,
            "runtime": self.runtime,
            "entrypoint": self.entrypoint,
            "metadata": metadata,
            "capabilities": self.capabilities,
            "permissions": self.permissions,
            "dependencies": self.dependencies,
            "compatible_host_versions": self.compatible_host_versions,
            "signature": self.signature,
            "healthcheck": self.healthcheck,
            "config_schema": self.config_schema,
            "ui_metadata": self.ui_metadata,
        }

    def write_toml(self, path: Path) -> None:
        lines = [
            f'id = "{self.id}"',
            f'version = "{self.version}"',
            f'api_version = "{self.api_version}"',
            f'runtime = "{self.runtime}"',
            f'entrypoint = "{self.entrypoint}"',
            "",
            "[metadata]",
            f'name = "{self.metadata.name}"',
            f'version = "{self.metadata.version}"',
            f'description = "{self.metadata.description}"',
            f'author = "{self.metadata.author or "unknown"}"',
            f'core_version = "{self.metadata.core_version or "0.1.0"}"',
            f"dependencies = {self.metadata.dependencies!r}",
            f"tags = {self.metadata.tags!r}",
        ]
        for capability in self.capabilities:
            lines.extend(
                [
                    "",
                    "[[capabilities]]",
                    f'id = "{capability.get("id", "")}"',
                    f'kind = "{capability.get("kind", "")}"',
                ]
            )
            for key in ("display_name", "description", "interface"):
                value = capability.get(key)
                if value is not None:
                    lines.append(f'{key} = "{value}"')
        for permission in self.permissions:
            lines.extend(
                [
                    "",
                    "[[permissions]]",
                    f'scope = "{permission.get("scope", "")}"',
                ]
            )
            if permission.get("resource") is not None:
                lines.append(f'resource = "{permission["resource"]}"')
            if permission.get("reason") is not None:
                lines.append(f'reason = "{permission["reason"]}"')
        if self.ui_metadata:
            lines.append("")
            lines.append("[ui_metadata]")
            for key, value in self.ui_metadata.items():
                lines.append(f'{key} = "{value}"')
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def build_manifest(
    metadata: PluginMetadata,
    entrypoint: str,
    runtime: str = "python",
) -> PluginManifest:
    """Build a manifest from metadata with sensible defaults."""
    return PluginManifest(
        id=metadata.name,
        version=metadata.version,
        api_version="v2",
        runtime=runtime,
        entrypoint=entrypoint,
        metadata=metadata,
        compatible_host_versions=[metadata.core_version or "0.1.0"],
    )


def scaffold_plugin_new(
    root: Path,
    plugin_name: str,
    *,
    author: str = "clawlegion",
    runtime: str = "python",
    entrypoint: str = "plugin_main.py",
) -> None:
    """Create a minimal protocol-first v2 plugin scaffold."""
    root.mkdir(parents=True, exist_ok=True)
    metadata = PluginMetadata(
        name=plugin_name,
        version="0.1.0",
        description=f"{plugin_name} plugin",
        author=author,
        core_version="0.1.0",
        tags=["plugin", runtime],
    )
    manifest = build_manifest(metadata, entrypoint=entrypoint, runtime=runtime)
    manifest.write_toml(root / "plugin.toml")
    if runtime == "python":
        (root / entrypoint).write_text(
            """#!/usr/bin/env python3\nimport json\nimport sys\n\nif \"--health\" in sys.argv:\n    print(\"ok\")\n    raise SystemExit(0)\n\nif \"--execute-llm-json\" in sys.argv:\n    payload = json.loads(sys.argv[sys.argv.index(\"--execute-llm-json\") + 1])\n    text = payload.get(\"messages\", [{}])[-1].get(\"content\", \"\")\n    print(json.dumps({\"text\": text, \"usage\": {\"prompt_tokens\": 1, \"completion_tokens\": 1, \"total_tokens\": 2}, \"finish_reason\": \"stop\"}))\n    raise SystemExit(0)\n\nprint(\"plugin runtime\")\n""",
            encoding="utf-8",
        )


class Plugin(ABC):
    """Base class for all ClawLegion plugins."""

    @abstractmethod
    def metadata(self) -> PluginMetadata:
        """Get plugin metadata."""
        pass

    @abstractmethod
    async def init(self, ctx: PluginContext) -> None:
        """Initialize the plugin.

        Args:
            ctx: Plugin context with configuration and directories.

        Raises:
            Exception: If initialization fails.
        """
        pass

    @abstractmethod
    async def shutdown(self) -> None:
        """Shutdown the plugin.

        Called when the plugin is being unloaded.
        Clean up any resources here.

        Raises:
            Exception: If shutdown fails.
        """
        pass

    async def enable(self) -> None:
        """Called when the plugin is enabled."""
        pass

    async def disable(self) -> None:
        """Called when the plugin is disabled."""
        pass

    async def on_config_reload(self, config: Dict[str, Any]) -> None:
        """Called when the plugin configuration is reloaded.

        Args:
            config: New plugin configuration.
        """
        pass

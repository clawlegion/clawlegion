# ClawLegion

ClawLegion is a plugin-driven multi-agent runtime for local demos and production-grade orchestration.

It is organized around four planes:
- Control plane: CLI, API, org/agent/plugin management
- Runtime plane: task execution, scheduling, message routing, memory
- Plugin plane: builtin and external capabilities with trust and signature rules
- UI plane: the web dashboard for live status and interaction

## What it is

ClawLegion is a framework for building and running coordinated agent systems. The v1 path is intentionally narrow: one runtime, one task model, one message bus, and a small set of builtin plugins.

## What it is not

It is not a finished enterprise autonomy product.
It is not a collection of placeholder agent types, or CLI wrappers.
It is not safe to ship with embedded secrets or business-specific defaults.

## Quick Start

1. Copy `.env.example` to `.env` and fill in the required values.
2. Start the local demo:

```bash
make bootstrap
make demo
```

3. Open the CLI, API, and web UI endpoints shown in the demo output.

If you only want the backend:

```bash
make dev-backend
```

If you only want the frontend:

```bash
make dev-web
```

## Architecture

See [docs/architecture.md](./docs/architecture.md) for the control plane, runtime plane, plugin plane, and UI plane boundaries.

## Core model

- `Task`: a unit of work with state, timeout, retry, and idempotency metadata
- `ExecutionContext`: runtime inputs, permissions, and tracing fields
- `MessageBus`: delivery for agent-to-agent and runtime events
- `Scheduler/Trigger`: time or event driven task submission
- `MemoryStore`: durable or demo memory backend

## Plugins

Builtin plugins are shipped for the supported runtime path.
Example plugins live separately from production plugins.
See the plugin docs once the runtime is bootstrapped.

## Configuration

Configuration is environment driven. Do not place real secrets, private endpoints, or company-specific defaults in tracked files.

- `.env.example`: local bootstrap template
- `clawlegion.toml`: safe checked-in defaults
- `config/org.toml`: neutral demo organization template

## Release

See [docs/release.md](./docs/release.md) for reproducible release requirements, artifacts, signing, SBOM, and rollback notes.

## Governance

- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)
- [SECURITY.md](./SECURITY.md)
- [CHANGELOG.md](./CHANGELOG.md)

## Languages

- [English](./README.md)
- [简体中文](./README_CN.md)
- [繁體中文](./README_TW.md)
- [Français](./README_FR.md)
- [Español](./README_ES.md)
- [Deutsch](./README_DE.md)
- [Русский](./README_RU.md)
- [العربية](./README_AR.md)
- [日本語](./README_JA.md)

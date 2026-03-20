# Architecture

ClawLegion is divided into five planes:

1. Control plane
   - CLI, API, org loading, plugin registration, status reporting
2. Runtime plane
   - task submission, scheduling, cancellation, event streaming, memory
3. Plugin plane
   - builtin tools and external capabilities with trust and signature rules
4. UI plane
   - dashboard views for agents, org structure, tasks, runs, and plugins
5. Packaging plane
   - releases, SDKs, signatures, SBOM, and published artifacts

## Dependency rules

- UI depends on API contracts, not internal runtime structures.
- Plugins depend on declared schemas and capability interfaces.
- Runtime depends on core task/message/memory abstractions.
- Control plane configures runtime, but does not fake runtime state.

## v1 path

The supported v1 path should be:

`config -> org -> runtime -> plugins -> message bus -> API -> UI -> shutdown`


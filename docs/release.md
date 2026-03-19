# Release

This repository should only publish from a reproducible source state.

## Preconditions

- Clean working tree
- Version updated in all release manifests
- Demo configuration uses placeholders only
- Tests and E2E checks passed
- Artifact signing keys available when required

## Artifacts

- GitHub release archives
- npm package
- PyPI package
- crates.io packages
- GitHub Pages site
- SBOM and checksum files

## Rollback

If a release is bad:

1. Stop advertising the release.
2. Mark the release as broken in the changelog.
3. Revoke or supersede published artifacts if the distribution channel supports it.
4. Cut a patch release from the last known good commit.


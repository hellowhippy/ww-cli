# Changelog

All notable changes to the public `ww-cli` / `whippy-workflows-cli` releases are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] — 2026-10-30

### Added

- Public AI skill `skills/building-workflow-apps` for Cursor / coding agents (sanitized app-building guide).
- Partner docs: `docs/BEST_PRACTICES.md` and `skills/README.md` (how to install and use the skill).
- Docs point to dashboard **Developers → API keys** (`https://app.whippy.co/developer/keys`) for `ww-cli login`.
- Getting Started includes a workflow builder screenshot so partners see where App steps appear.

## [0.1.2] — 2026-10-30

### Fixed

- Connection-error tip now references `https://workflows.whippy.ai` (and localhost) instead of an
  outdated Render hostname.

## [0.1.1] — 2026-10-30

### Fixed

- Document first publish as `ww-cli push --create` (plain `push` requires an existing app).
- Harden public-mirror CI: version-bump gate only on `src/` / lockfile changes; honor
  `create_release=false` on workflow_dispatch; skip invalid `WW_CLI_SYNC_TOKEN` when resolving
  auth; scan mirror for accidental real credentials before publish.

## [0.1.0] — 2026-10-30

### Added

- First public mirror of the Whippy Workflows CLI (`ww-cli` / `whippy-workflows-cli`).
- External install via `cargo install --git https://github.com/hellowhippy/ww-cli`.
- Getting-started docs for partners building workflow apps against `https://workflows.whippy.ai`.

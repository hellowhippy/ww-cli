# Whippy Workflows CLI (`ww-cli`)

Build, validate, and publish **Whippy workflow apps** (integrations) from your own machine.

You do **not** need the Whippy monorepo. You write a small TypeScript package locally; the CLI talks to Whippy’s hosted workflows platform at `https://workflows.whippy.ai`.

## Install

### Recommended: Cargo (from this repo)

Requires [Rust](https://rustup.rs/) and [Node.js](https://nodejs.org/) (the CLI uses Node for bundling).

```bash
cargo install --git https://github.com/hellowhippy/ww-cli --locked
```

This installs two binaries on your `PATH` (usually `~/.cargo/bin`):

- `ww-cli` — short name
- `whippy-workflows-cli` — full name

```bash
ww-cli --help
```

### From a release binary

When GitHub Releases are published for a version, download the archive for your OS/arch from:

https://github.com/hellowhippy/ww-cli/releases

Unpack and put `ww-cli` on your `PATH`.

### Upgrade

```bash
cargo install --git https://github.com/hellowhippy/ww-cli --locked --force
```

## Prerequisites

1. **A Whippy account + API key**  
   Create an API key in the Whippy dashboard. If you do not have a Whippy account, contact [whippy.ai](https://whippy.ai) / Whippy sales — there is no anonymous sandbox for publishing apps.

2. **Node.js** (for `build` / bundling)

3. Optional: **Wrangler** for local `invoke` against a Worker  
   ```bash
   npm install -g wrangler@3.114.1
   ```
   (Pinning Wrangler 3.x avoids known sharp/Node issues with Wrangler 4 on some setups.)

## Quick start

```bash
ww-cli login
# paste your Whippy API key when prompted

ww-cli init my-app --language typescript --auth api_key
cd my-app

# edit src/** …

ww-cli build
ww-cli validate
ww-cli invoke <action_key> --data test-data.json   # after filling auth in test-data.json

# When you are ready to publish a version (humans only — agents should not push):
ww-cli push
ww-cli promote <semver>
```

Default API base URL is **`https://workflows.whippy.ai`**. You normally never need to change it.

## Core commands

| Command | Purpose |
|--------|---------|
| `ww-cli login` | Store API key (+ API URL) in `~/.whippyrc` |
| `ww-cli logout` | Clear stored credentials |
| `ww-cli init <name>` | Scaffold a new workflow app |
| `ww-cli build` | Compile bundle + `app.json` |
| `ww-cli validate` | Local structural checks |
| `ww-cli invoke <key>` | Live-test one action locally |
| `ww-cli push` | Create/update an app version on Whippy |
| `ww-cli promote <ver>` | Publish a draft version |
| `ww-cli versions` | List remote versions |
| `ww-cli logo` | Upload app logo / brand color |

Run `ww-cli <command> --help` for flags.

## Auth note

`ww-cli login` persists `api_url` in `~/.whippyrc`. If you ever pointed the CLI at a non-production URL, later commands keep using it until you fix or delete `~/.whippyrc` and log in again against production.

## Docs

- [Getting started](./docs/GETTING_STARTED.md) — longer walkthrough and gotchas
- [Changelog](./CHANGELOG.md)

## Source of truth

This public repository is a **mirror** of `workflows/cli` inside Whippy’s private monorepo. Releases and docs here are updated automatically when that path changes on `main`. Do not open feature PRs here expecting them to merge back — contribute through Whippy.

## License

See [LICENSE](./LICENSE).

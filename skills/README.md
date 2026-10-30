# AI skills for building Whippy workflow apps

These folders follow the [Agent Skills](https://docs.cursor.com/context/skills) convention (`SKILL.md` with YAML frontmatter). Coding agents (Cursor, Claude Code, Codex, and similar tools that load project skills) can use them so they follow Whippy’s app contract instead of inventing patterns.

## What’s included

| Skill | Use when |
|-------|----------|
| [`building-workflow-apps`](./building-workflow-apps/SKILL.md) | Creating or editing a Whippy workflow app with `ww-cli` |

Human-readable companion docs (not agent skills):

- [Getting started](../docs/GETTING_STARTED.md)
- [Best practices](../docs/BEST_PRACTICES.md)

## How to use with Cursor

### Option A — Copy into your app repo (recommended)

From your workflow app root (the folder with `src/index.ts`):

```bash
mkdir -p .cursor/skills
cp -R /path/to/ww-cli/skills/building-workflow-apps .cursor/skills/
```

Or if you cloned this repo:

```bash
git clone https://github.com/hellowhippy/ww-cli.git /tmp/ww-cli
mkdir -p .cursor/skills
cp -R /tmp/ww-cli/skills/building-workflow-apps .cursor/skills/
```

Cursor discovers skills under `.cursor/skills/*/SKILL.md`. Start a chat in the app and ask to build or fix an action — the agent should load **building-workflow-apps** from the description.

### Option B — Point the agent at the file

In chat:

> Read `skills/building-workflow-apps/SKILL.md` from https://github.com/hellowhippy/ww-cli (or a local clone) and follow it while building this Whippy workflow app.

### Option C — Personal / user skills

Copy `building-workflow-apps` into your Cursor user skills directory so it applies across projects (path varies by Cursor version; typically under your user Agent skills / `.cursor/skills` in the home config). Prefer Option A so the skill version travels with the app repo.

## How to use with Claude Code / other agents

1. Clone or download this repository.
2. Add the skill path to the agent’s skill/plugin load path, **or** paste / `@`-mention `skills/building-workflow-apps/SKILL.md` at the start of the task.
3. Tell the agent: *Follow the Whippy workflow-apps skill. Agents may run `ww-cli build`, `validate`, and `invoke` only — never `push` / `promote` unless I ask.*

## Rules of the road for agents

- **Agents test; humans deploy.** Allowed: `build`, `validate`, `invoke`, `test`. Not allowed unless the user asks: `push`, `promote`, `make-global`, `login`, `secrets set`.
- Target production API `https://workflows.whippy.ai` via `ww-cli login` (no local Whippy stack required).
- Whippy API keys: dashboard **Developers → API keys** → [https://app.whippy.co/developer/keys](https://app.whippy.co/developer/keys).
- Prefer `--auth custom` for API-key apps so the Connect form renders.
- First publish: `ww-cli push --create`; later: `ww-cli push`.

## Keeping skills up to date

This public repo is mirrored from Whippy’s private monorepo. Re-copy the `skills/` folder after upgrading `ww-cli` or when a release notes a docs/skills update.

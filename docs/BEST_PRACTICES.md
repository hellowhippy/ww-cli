# Best practices for Whippy workflow apps

Practical rules for shipping reliable apps with [`ww-cli`](https://github.com/hellowhippy/ww-cli). For agents, also load the [building-workflow-apps skill](../skills/building-workflow-apps/SKILL.md).

## Product mindset

1. **Design for the builder UI.** Users pick actions, fill forms, and map `${step.output.data…}` paths. Friendly labels, dropdowns for IDs, and accurate samples beat clever code.
2. **Ship useful fields.** Don’t stop at the vendor’s minimum required body — expose the optional fields people need for real workflows (status, assignee, filters, pagination, etc.).
3. **Prefer OpenAPI coverage.** When a machine-readable spec exists, inventory endpoints and convert the useful ones. Document intentional skips with a reason.
4. **Hero actions first, then finish.** Prove connection + 1–2 core loops early, then complete remaining endpoints rather than leaving a thin “demo” forever.

## Auth

- Your **Whippy** API key (for `ww-cli login`) lives under **Developers → API keys** in the dashboard:  
  [https://app.whippy.co/developer/keys](https://app.whippy.co/developer/keys).
- API-key **workflow apps**: authentication `type: 'custom'` with an `api_key` (or similar) field — **not** scaffold `api_key` type (Connect form won’t render).
- Implement a cheap real `authentication.test`.
- OAuth: confirm Whippy’s production callback URL with Whippy before registering the vendor app; store client secrets with `ww-cli secrets` (never commit them).

## Code layout

- One handler per file; filename = action `key`.
- Reads → `searches/`; durable mutations → `creates/`.
- Shared logic in `src/lib/` only.
- Cloudflare Worker rules: no `Buffer`, `process`, `require`, or Node builtins — use `w.request` and Web APIs.

## Fields and validation

- Every input needs `help_text` and a label ≥ 3 characters.
- Static enums: `choices: { value: 'Label' }` objects (never arrays).
- IDs users shouldn’t type: hidden `list_*` **search** + `dynamic: 'list_….id.name'`.
- Prefer push-safe types: avoid `type: 'array'` / `'select'` if `push` 422s; use string choices or JSON text instead.
- No unknown keys like `placeholder` on fields — put examples in `help_text`.

## Outputs and samples

- Searches return **arrays**; creates return **objects**.
- Keep source samples **unwrapped**; let `ww-cli build` wrap under `data`.
- Base samples on vendor docs or live responses — never invent filler IDs.
- Return the record users will map next (id, url, status), not only `{ success: true }`.

## Deploy and versions

- First publish: `ww-cli push --create`. Commit `.whippyapprc`.
- Later: plain `ww-cli push`. Never `--create` twice for the same app.
- Promote when ready; **re-add** app steps in test workflows (steps pin to version IDs).
- Bump `package.json` version when shipping a new published version; keep `src/index.ts` aligned.

## Testing

- `build` → `validate` → `invoke` with real `test-data.json` (gitignored).
- Run a builder Step test after promote.
- Surface vendor error bodies in thrown errors.

## Security & hygiene

- Never commit `test-data.json`, `secrets.json`, or API keys.
- Don’t log raw credentials.
- Treat `~/.whippyrc` carefully: a wrong stored `api_url` sends pushes to the wrong environment — re-run `ww-cli login` against production (`https://workflows.whippy.ai`) if something 404s unexpectedly.

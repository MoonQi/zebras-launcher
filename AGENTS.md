# Repository Guidelines

## Project Structure & Module Organization
The TypeScript/React UI lives under `src` with feature folders: `components` (UI primitives, workspace cards, logs), `contexts` for shared state, `hooks` (e.g. `useWorkspace`), `services/tauri.ts` for bridge calls, and typed contracts in `src/types`. Global styling is concentrated in `src/styles/index.css`. Desktop runtime code lives in `src-tauri`, where `src/commands` exposes workspace/project operations, `src/services` hosts parsers and process managers, and configuration/static assets reside in `tauri.conf.json` and `icons/`. Built assets land in `dist/`; avoid editing `node_modules/` or `src-tauri/target/`.

## Build, Test, and Development Commands
- `npm install` - install Node dependencies (Vite, React, Tauri CLI bindings).
- `npm run dev` - start the Vite dev server for the web UI only.
- `npm run tauri dev` - launch the full Tauri desktop shell with the Rust backend.
- `npm run build` - TypeScript check via `tsc` then bundle into `dist/`.
- `npm run preview` - serve the latest `dist/` output for smoke-testing.
- `cargo test` (inside `src-tauri`) - run backend unit tests; add cases beside the modules they cover.
- `cargo fmt && cargo clippy -- -D warnings` - keep Rust code formatted and lint-clean.

## Coding Style & Naming Conventions
Code is TypeScript-first with `strict` compiler settings; keep props and hook return types explicit and colocate them in `src/types`. Use 2-space indentation, `PascalCase` React components, `camelCase` utilities, and prefix hooks with `use`. Favor functional, side-effect-free components and isolate bridge calls to `src/services/tauri.ts`. Keep CSS variables and layout rules centralized in `src/styles/index.css`.

## Testing Guidelines
Frontend tests are not scaffolded yet; prefer colocated `*.test.tsx` files that exercise hooks/components with React Testing Library once added. Until then, document manual scenarios (workspace discovery, launch/stop flows) in the PR. Rust modules should define `#[cfg(test)] mod tests` next to the implementation so `cargo test` stays fast; mock filesystem and port interactions via the helpers in `src-tauri/src/services`. Aim for coverage of parsers (`config_parser.rs`, `workspace_list.rs`) and process orchestration.

## Commit & Pull Request Guidelines
Use Conventional Commit subjects (e.g., `feat(workspace): add state caching`) to keep history searchable even though the .git folder is not vendored here. Each PR must include: a concise summary, linked issue/task ID, screenshots or terminal output for UX-affecting work, and a checklist showing `npm run build`, `npm run tauri dev`, and `cargo test` were exercised. Squash fixups locally before review.

## Security & Configuration Tips
The runtime configuration and bundler metadata live in `src-tauri/tauri.conf.json`; do not hard-code secrets or paths inside React components. Instead, ask for them via secure dialogs or read-only config files parsed by `src-tauri/src/services/config_parser.rs`. Review file-system and shell permissions before adding new Tauri commands, and prefer using the helpers in `src/services/tauri.ts` so every bridge call remains centralized and auditable.

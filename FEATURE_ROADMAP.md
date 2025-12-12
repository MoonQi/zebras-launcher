# Zebras Launcher - Feature Extension Analysis & Recommendations

## Current State Overview

Zebras Launcher is a Tauri-based desktop application for managing Zebras micro-frontend projects. It provides:
- Multi-workspace project organization
- Process lifecycle management (start/stop/restart)
- Port conflict resolution
- Debug dependency configuration
- Real-time log streaming
- npm/pnpm task execution
- Automatic project discovery

## Core Architecture

**Frontend:** React + TypeScript with Tauri integration
**Backend:** Rust with Tauri framework
- 22 Tauri commands across 5 categories
- 6 core services (ProcessManager, PortManager, ProjectScanner, ConfigParser, WorkspaceService, WorkspaceList)
- Centralized config storage in `~/.zebras-launcher/`

**Key Files:**
- `/src/components/project/ProjectCard.tsx` - Main project UI
- `/src/components/workspace/WorkspaceManager.tsx` - Workspace operations
- `/src-tauri/src/services/process_manager.rs` - Process management
- `/src-tauri/src/services/config_parser.rs` - Config parsing

---

## Recommended Feature Extensions (Prioritized)

### 🔥 HIGH PRIORITY - Core Developer Workflow

#### 1. **Git Integration** (Your Example)
**Status Display:**
- Current branch name badge on each ProjectCard
- Visual indicator: ahead/behind remote (e.g., "↑3 ↓2")
- Uncommitted changes count (modified, staged, untracked)
- Dirty/clean state indicator

**Operations:**
- `git pull` button for each project
- `git fetch` to refresh status
- Pull all projects in workspace (batch operation)
- Show last commit message tooltip

**Implementation:**
- Backend: New `git_commands.rs` module using `std::process::Command`
- Parse `git status --porcelain -b` for status
- Parse `git rev-list --left-right --count @{upstream}...HEAD` for ahead/behind
- Frontend: GitStatus component in ProjectCard header
- New Tauri commands: `get_git_status`, `git_pull_project`, `git_fetch_project`

**Why:** Developers constantly switch contexts. Seeing git status at a glance prevents "which branch am I on?" and "did I pull latest?" issues.

---

#### 2. **Environment Variable Management**
**Display:**
- Show .env file presence/absence indicator
- Quick view of current environment variables
- Multiple .env profile switching (.env.local, .env.development, .env.production)

**Operations:**
- Create/edit .env files in UI
- Toggle between .env profiles
- Validate required variables (based on .env.example)
- Copy .env.example to .env.local

**Implementation:**
- Backend: New `env_manager.rs` service
- Parse .env files, track multiple profiles
- Commands: `list_env_files`, `read_env_file`, `write_env_file`, `switch_env_profile`
- Frontend: EnvManager modal component

**Why:** Micro-frontends often need different API endpoints/keys per environment. Manual .env editing is error-prone.

---

#### 3. **Dependency Health Monitoring**
**Display:**
- Outdated package count badge (e.g., "5 updates")
- Security vulnerability count (npm audit)
- Lock file status (package-lock.json vs package.json sync)

**Operations:**
- Run `npm outdated` per project
- Run `npm audit` for security scan
- Quick update buttons (update all, update major, update minor)
- View dependency diff before updating

**Implementation:**
- Backend: New commands `check_outdated_deps`, `check_security_audit`
- Parse npm JSON output
- Frontend: DependencyHealth component in ProjectCard

**Why:** Keeping dependencies updated is critical but often forgotten. Visual indicators make it actionable.

---

#### 4. **Build & Test Integration**
**Operations:**
- Run `npm run build` (with build output)
- Run `npm test` or `npm run test`
- Display build status (success/failure)
- Build time tracking
- Test coverage percentage

**Display:**
- Build status badge (built/failed/not built)
- Last build time
- Build size (if applicable)
- Test pass/fail counts

**Implementation:**
- Extend ProcessManager to support build/test tasks
- Parse test output for pass/fail counts
- Commands: `run_build`, `run_tests`
- Frontend: BuildStatus component

**Why:** Quick sanity checks before deploying. Currently limited to deploy/start, but build/test are fundamental.

---

#### 5. **Health Monitoring for Running Projects**
**Auto Health Checks:**
- Ping HTTP endpoints when projects start (e.g., `http://localhost:{port}/health`)
- Display response status (200 OK, 500 Error, timeout)
- Auto-restart on crash detection
- Response time tracking

**Display:**
- Health status indicator (green/yellow/red)
- Uptime counter
- Last health check timestamp
- Resource usage (if feasible via OS APIs)

**Implementation:**
- Backend: New `health_monitor.rs` service
- Periodic health checks via tokio::spawn
- Commands: `get_health_status`, `configure_health_endpoint`
- Frontend: HealthIndicator in ProjectCard status area

**Why:** Running != Working. Knowing if localhost:3000 is actually responding prevents "why isn't it loading?" debugging.

---

### 🟡 MEDIUM PRIORITY - Enhanced Productivity

#### 6. **Editor Integration**
**Operations:**
- "Open in VS Code" button per project
- "Open in Terminal" button
- "Open in File Explorer"
- Configurable editor preference (VS Code, WebStorm, Cursor, etc.)

**Implementation:**
- Backend: Use `std::process::Command` to spawn `code {path}` or `open -a "Visual Studio Code" {path}`
- Commands: `open_in_editor`, `open_in_terminal`, `open_in_explorer`
- Frontend: Action buttons in ProjectCard header

**Why:** Constant context switching. One-click access to editor/terminal saves time.

---

#### 7. **Enhanced Logging**
**Features:**
- Log persistence (save last 1000 lines to file)
- Log search/filter (regex search)
- Log export (save to .txt)
- Error highlighting (stack traces, error keywords)
- Log grouping (collapse repeated messages)
- Timestamp display
- Log levels (info/warn/error filtering)

**Implementation:**
- Backend: Buffer logs in memory, write to `~/.zebras-launcher/logs/{project_id}.log`
- Commands: `search_logs`, `export_logs`, `clear_logs`
- Frontend: Enhanced LogViewer component with search bar

**Why:** Current logs are ephemeral. Need to find specific errors or review past runs.

---

#### 8. **Custom Task/Script Management**
**Features:**
- Detect all npm scripts from package.json
- Display custom scripts in dropdown menu
- Run any script (not just hardcoded install/deploy/start)
- Save favorite scripts per project
- Add custom shell commands (non-npm tasks)

**Implementation:**
- Backend: Parse package.json `scripts` section
- Commands: `list_npm_scripts`, `run_custom_script`
- Frontend: ScriptMenu dropdown in ProjectCard

**Why:** Projects have many scripts (lint, format, storybook, e2e). Hardcoding limits flexibility.

---

#### 9. **Project Templates & Scaffolding**
**Features:**
- Clone existing project as template
- Quick new project creation from Zebras boilerplate
- Configuration presets (common settings)
- Automatic workspace addition after creation

**Implementation:**
- Backend: File copy operations, git clone support
- Commands: `create_project_from_template`, `list_templates`
- Frontend: New "Create Project" modal in WorkspaceManager

**Why:** Spinning up new micro-frontends is common. Consistency through templates.

---

#### 10. **Port Management Enhancements**
**Features:**
- Port usage history (which project used which port)
- Custom port assignment (not just auto-resolve)
- Port range configuration per workspace
- Detect port conflicts before starting (not just on conflict)
- Show which process is using a port (OS-level check)

**Implementation:**
- Enhance PortManager to track history
- Add `lsof` or `netstat` integration for OS-level port checks
- Commands: `get_port_usage`, `assign_custom_port`

**Why:** Current system auto-resolves, but sometimes developers want specific ports.

---

### 🟢 NICE-TO-HAVE - Advanced Features

#### 11. **Workspace Import/Export**
- Export workspace config to JSON (shareable with team)
- Import workspace from JSON
- Sync workspace configs via Git
- Workspace templates

**Why:** Team onboarding. Share exact setup with new developers.

---

#### 12. **Notification System**
- Desktop notifications for events (build complete, deploy finished, error occurred)
- Configurable notification rules
- Webhook support (Slack, Discord)

**Why:** Long-running tasks. Get notified when deploy finishes instead of watching.

---

#### 13. **API Testing Tools**
- Quick HTTP request tester
- Save common requests per project
- Test API endpoints directly from UI
- View request/response

**Why:** Micro-frontends interact with APIs. Quick API testing without Postman.

---

#### 14. **Dependency Graph Visualization**
- Visual graph of debug dependencies
- Show which projects depend on which
- Detect circular dependencies
- One-click navigation between dependent projects

**Why:** Complex micro-frontend setups. Understanding project relationships.

---

#### 15. **Performance Monitoring**
- Build time tracking over time
- Bundle size tracking
- Compare bundle sizes across commits
- Lighthouse score integration

**Why:** Performance budgets. Prevent bundle bloat.

---

#### 16. **Docker/Container Support**
- Detect docker-compose.yml
- Quick docker-compose up/down
- Container status display
- Container logs

**Why:** Some projects use databases or services in containers.

---

#### 17. **Database Tools** (if applicable)
- Database connection status
- Quick SQL query runner
- Migration status
- Seed data management

**Why:** Full-stack projects need database management.

---

## Implementation Priority Matrix

| Feature | User Impact | Dev Effort | Priority |
|---------|------------|------------|----------|
| Git Integration | Very High | Medium | **P0** |
| Health Monitoring | High | Medium | **P0** |
| Build & Test | High | Low | **P0** |
| Custom Scripts | High | Low | **P0** |
| Env Management | Medium | Medium | P1 |
| Dependency Health | Medium | Medium | P1 |
| Editor Integration | High | Low | P1 |
| Enhanced Logging | Medium | High | P2 |
| Port Enhancements | Low | Low | P2 |
| Workspace Import/Export | Low | Low | P3 |
| API Testing | Medium | High | P3 |
| Notifications | Low | Medium | P3 |

---

## Recommended First Phase

Based on highest impact and lowest effort:

1. **Git Status Display** - Branch, ahead/behind, dirty state (2-3 days)
2. **Git Pull** - Per-project and bulk pull (1 day)
3. **Custom Script Runner** - Show all npm scripts (1 day)
4. **Build/Test Integration** - Run build/test commands (1-2 days)
5. **Editor Integration** - Open in VS Code (0.5 days)

Total: ~1 week of development for massive productivity boost.

---

---

## User Preferences (Confirmed)

✅ **Priority 1:** Git Integration (status display, pull operations, fetch & refresh)
✅ **Priority 2:** Editor Integration (open in VS Code, terminal, file explorer)
✅ **Auto-restart:** No - just show health status, don't auto-restart projects

---

## Recommended Implementation Plan

### Implementation Scope

**Phase 1: Git Integration**
- Git status display with badges (branch, ahead/behind, uncommitted changes)
- Git pull per project (skip if uncommitted changes)
- Git fetch per project
- Bulk git pull/fetch across workspace
- Handle edge cases: not a git repo, git not installed, merge conflicts, network failures

**Phase 2: Editor Integration**
- Open project in configured editor (VS Code default)
- Open project in terminal
- Open project in file explorer/finder
- Configurable editor preference in workspace settings
- Cross-platform support (Windows/macOS/Linux)

---

### Architecture Approach

Following existing patterns in the codebase:

**Backend (Rust):**
1. New models in `/src-tauri/src/models/`:
   - `git_status.rs` - GitStatus, GitPullResult, BulkGitResult
   - `editor_config.rs` - EditorType, EditorConfig

2. New services in `/src-tauri/src/services/`:
   - `git_service.rs` - Git command execution using `std::process::Command`
   - `editor_service.rs` - Editor/terminal/explorer launching with cross-platform support

3. New commands in `/src-tauri/src/commands/`:
   - `git.rs` - Expose git operations to frontend
   - `editor.rs` - Expose editor operations to frontend

4. Update `WorkspaceSettings` in `/src-tauri/src/models/workspace.rs` to include `EditorConfig`

**Frontend (React/TypeScript):**
1. New types in `/src/types/`:
   - `git.ts` - TypeScript interfaces matching Rust models
   - `editor.ts` - Editor configuration types

2. New components in `/src/components/`:
   - `project/GitStatusBadge.tsx` - Display branch, ahead/behind, uncommitted counts
   - `workspace/EditorSettings.tsx` - Configure preferred editor

3. Modify existing components:
   - `project/ProjectCard.tsx` - Add GitStatusBadge, git buttons (fetch/pull), editor buttons
   - `workspace/WorkspaceManager.tsx` - Add bulk git operations (Fetch All, Pull All)
   - `App.tsx` - Add handlers for workspace-level git operations

4. Update `/src/services/tauri.ts` with new API functions

---

### Key Features & UI Design

#### Git Status Display (in ProjectCard header)
```
[Project Name] [Running Badge]
[V3] [子应用]
[🌿 main] [✎ 2] [↓3] [↑1]
   ↑        ↑     ↑    ↑
Branch  Uncommitted Behind Ahead
```

**Color coding:**
- Branch: Blue badge
- Uncommitted: Yellow/orange badge
- Behind: Green badge (updates available)
- Ahead: Gray badge (local commits)

#### Git Actions (in ProjectCard)
```
[npm i] [pnpm i] [npm run deploy]
────────────────────────────────
[Git Fetch] [Git Pull]
```

- Pull disabled if uncommitted changes
- Loading states during operations

#### Workspace Bulk Operations (in WorkspaceManager)
```
[全部启动] [全部停止] [Git Fetch All] [Git Pull All]
```

Shows result summary:
- Succeeded projects
- Skipped projects (uncommitted changes)
- Failed projects with errors

#### Editor Actions (in ProjectCard)
```
[< > Editor] [>_ Terminal] [📁 Explorer]
```

Three quick-access buttons for common workflows.

---

### Critical Files to Modify

**Backend:**
- `/src-tauri/src/models/git_status.rs` (NEW)
- `/src-tauri/src/models/editor_config.rs` (NEW)
- `/src-tauri/src/services/git_service.rs` (NEW)
- `/src-tauri/src/services/editor_service.rs` (NEW)
- `/src-tauri/src/commands/git.rs` (NEW)
- `/src-tauri/src/commands/editor.rs` (NEW)
- `/src-tauri/src/models/workspace.rs` (MODIFY - add EditorConfig to WorkspaceSettings)
- `/src-tauri/src/models/mod.rs` (MODIFY - export new models)
- `/src-tauri/src/services/mod.rs` (MODIFY - export new services)
- `/src-tauri/src/commands/mod.rs` (MODIFY - export new commands)
- `/src-tauri/src/main.rs` (MODIFY - register new commands)

**Frontend:**
- `/src/types/git.ts` (NEW)
- `/src/types/editor.ts` (NEW)
- `/src/components/project/GitStatusBadge.tsx` (NEW)
- `/src/components/workspace/EditorSettings.tsx` (NEW)
- `/src/components/project/ProjectCard.tsx` (MODIFY - integrate git status + buttons)
- `/src/components/workspace/WorkspaceManager.tsx` (MODIFY - add bulk git buttons)
- `/src/App.tsx` (MODIFY - add bulk git handlers)
- `/src/services/tauri.ts` (MODIFY - add new API functions)
- `/src/types/index.ts` (MODIFY - export new types)
- `/src/types/workspace.ts` (MODIFY - add EditorConfig to WorkspaceSettings)

---

### Edge Cases Handled

**Git Integration:**
1. Project not in git repo → No git UI shown
2. Git not installed → Warning banner, features disabled
3. Merge conflicts during pull → Error message with conflict details
4. Network failures → Clear error, retry option
5. Uncommitted changes → Pull disabled, skip during bulk operations
6. No upstream configured → No ahead/behind counts shown
7. Detached HEAD → Show "(detached)" as branch

**Editor Integration:**
1. Editor not installed → Helpful error message with instructions
2. Editor not in PATH → Suggest adding to PATH or custom command
3. Terminal not found (Linux) → Try multiple terminals, show error if all fail
4. Special characters in paths → Proper escaping handled automatically

---

### Implementation Timeline

**Estimated: 6-7 days**

- Days 1-2: Git backend (models, service, commands)
- Days 3-4: Git frontend (UI components, integration)
- Day 5: Editor backend + frontend
- Day 6-7: Testing, polish, cross-platform verification

---

### Technical Patterns

**Following existing codebase patterns:**
- Git commands use `std::process::Command` (sync) and `tokio::process::Command` (async), similar to `ProcessManager`
- Cross-platform handling via `#[cfg(target_os = "...")]` conditional compilation
- Commands are thin wrappers delegating to service layer
- Frontend uses `invoke()` from `@tauri-apps/api/tauri`
- State management via React hooks
- No new dependencies required (uses existing Tauri, tokio, serde)

---

## Next Steps

Once approved, implementation will proceed in phases:
1. Git Integration (backend + frontend)
2. Editor Integration (backend + frontend)
3. Testing & polish across platforms

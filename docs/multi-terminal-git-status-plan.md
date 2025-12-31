# ProjectCard 多终端 & Git 状态功能实现计划

## 功能概述

### 功能 1: 多终端支持
- 每个 ProjectCard 最多可打开 **3 个终端标签**
- 用户可输入**自定义命令**（在项目目录下独立执行）
- 保留现有快捷按钮（npm i, pnpm i, deploy）
- 每个终端有独立的日志输出，支持 Kill/Clear

### 功能 2: Git 分支状态
- 显示当前 **git 分支名称**
- **每 15 分钟**自动 git fetch 检查远程更新
- 显示更新徽章（如 `↓3 可拉取`）
- 一键 Pull 按钮（有未提交更改时禁用）
- **可配置**系统通知提醒

---

## 文件变更清单

### 新增文件 (Backend - Rust)

| 文件 | 用途 |
|------|------|
| `src-tauri/src/models/terminal.rs` | TerminalSession, TerminalStatus 结构体 |
| `src-tauri/src/models/git_status.rs` | GitStatus, GitPullResult 结构体 |
| `src-tauri/src/services/terminal_manager.rs` | 终端会话管理、命令执行、日志流 |
| `src-tauri/src/services/git_manager.rs` | Git 操作（status, fetch, pull） |
| `src-tauri/src/commands/terminal.rs` | Tauri 终端命令 |
| `src-tauri/src/commands/git.rs` | Tauri Git 命令 |

### 新增文件 (Frontend - TypeScript/React)

| 文件 | 用途 |
|------|------|
| `src/types/terminal.ts` | 终端类型定义 |
| `src/types/git.ts` | Git 类型定义 |
| `src/types/settings.ts` | 应用设置类型 |
| `src/components/project/TerminalPanel.tsx` | 多终端 UI 组件 |
| `src/components/settings/SettingsPanel.tsx` | 通知设置面板 |
| `src/hooks/useGitStatus.ts` | Git 状态管理 + 定时 fetch |
| `src/hooks/useAppSettings.ts` | 应用设置 (localStorage) |

### 修改文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/models/mod.rs` | 导出 terminal, git_status |
| `src-tauri/src/services/mod.rs` | 导出 terminal_manager, git_manager |
| `src-tauri/src/services/process_manager.rs` | LogMessage 添加 session_id 字段 |
| `src-tauri/src/commands/mod.rs` | 导出 terminal, git |
| `src-tauri/src/state.rs` | AppState 添加 terminal_manager |
| `src-tauri/src/main.rs` | 注册新命令 |
| `src/types/index.ts` | 导出新类型 |
| `src/services/tauri.ts` | 添加终端和 Git API |
| `src/components/project/ProjectCard.tsx` | 集成 TerminalPanel 和 Git 状态 |
| `src/App.tsx` | 添加设置状态，传递 gitStatuses |

---

## 实现步骤

### Phase 1: 多终端 Backend

1. **创建 `src-tauri/src/models/terminal.rs`**
   ```rust
   pub struct TerminalSession {
       pub session_id: String,
       pub project_id: String,
       pub command: Option<String>,
       pub status: TerminalStatus,  // Idle | Running | Completed | Error
       pub pid: Option<u32>,
   }
   ```

2. **创建 `src-tauri/src/services/terminal_manager.rs`**
   - `create_session(project_id)` - 创建终端（检查 < 3）
   - `run_command(session_id, project_path, command)` - 执行命令
   - `kill_session(session_id)` - 终止进程
   - `close_session(session_id)` - 关闭终端
   - 使用 `sh -c "cmd"` (Unix) / `cmd /C "cmd"` (Windows)
   - 发送事件 `terminal_log` 带 `session_id`

3. **创建 `src-tauri/src/commands/terminal.rs`**
   - `create_terminal_session`
   - `run_terminal_command`
   - `kill_terminal_session`
   - `close_terminal_session`
   - `get_terminal_sessions`

4. **更新 state.rs 和 main.rs**

### Phase 2: 多终端 Frontend

5. **创建 `src/types/terminal.ts`**

6. **更新 `src/services/tauri.ts`** 添加终端 API

7. **创建 `src/components/project/TerminalPanel.tsx`**
   ```tsx
   // UI 结构
   [Tab 1] [Tab 2] [+]           // 标签栏
   ─────────────────────
   > [命令输入框] [Run] [Kill]   // 输入区
   ─────────────────────
   [日志输出区域]                // session 过滤
   ```
   - 状态：sessions[], activeSessionId, sessionLogs Map
   - 监听 `terminal_log` 事件，按 session_id 过滤

8. **修改 `src/components/project/ProjectCard.tsx`**
   - 添加终端面板切换按钮
   - 条件渲染 `<TerminalPanel />`

### Phase 3: Git Backend

9. **创建 `src-tauri/src/models/git_status.rs`**
   ```rust
   pub struct GitStatus {
       pub branch: Option<String>,
       pub has_remote: bool,
       pub uncommitted_count: u32,
       pub ahead_count: u32,
       pub behind_count: u32,
   }
   ```

10. **创建 `src-tauri/src/services/git_manager.rs`**
    - `get_status(path)` - 解析 git 状态
      - `git rev-parse --abbrev-ref HEAD` (分支)
      - `git status --porcelain` (未提交数)
      - `git rev-list --count --left-right @{u}...HEAD` (ahead/behind)
    - `fetch(path)` - `git fetch --quiet`
    - `pull(path)` - `git pull --ff-only`
    - `is_git_repo(path)` - 检查 .git 目录

11. **创建 `src-tauri/src/commands/git.rs`**
    - `get_git_status`
    - `git_fetch`
    - `git_pull`
    - `is_git_repo`

### Phase 4: Git Frontend

12. **创建 `src/types/git.ts` 和 `src/types/settings.ts`**

13. **创建 `src/hooks/useGitStatus.ts`**
    - 初始加载所有项目 git 状态
    - `setInterval` 每 15 分钟 fetch + 更新状态
    - 检测到新 behind_count 时触发通知

14. **创建 `src/hooks/useAppSettings.ts`**
    - 从 localStorage 读写设置
    - `gitFetchInterval`, `gitNotificationsEnabled`

15. **修改 `src/components/project/ProjectCard.tsx`**
    ```tsx
    // Header 区域添加
    [🌿 main] [✎2] [↓3 可拉取]

    // Actions 区域添加
    [Fetch] [Pull]  // Pull 在有未提交时禁用
    ```

16. **创建 `src/components/settings/SettingsPanel.tsx`**
    - Git 通知开关
    - Fetch 间隔设置

17. **修改 `src/App.tsx`**
    - 使用 `useGitStatus` hook
    - 传递 gitStatuses 到 ProjectGrid

---

## 关键实现细节

### 终端命令执行
```rust
// Unix
Command::new("sh").args(["-c", &command]).current_dir(&project_path)

// Windows
Command::new("cmd").args(["/C", &command]).current_dir(&project_path)
```

### Git 状态解析
```rust
// 分支名
git rev-parse --abbrev-ref HEAD

// 未提交文件数
git status --porcelain | 统计行数

// ahead/behind
git rev-list --count --left-right @{u}...HEAD
// 输出: "3\t5" 表示 ahead 3, behind 5
```

### 前端定时器
```typescript
useEffect(() => {
  const id = setInterval(fetchAndNotify, 15 * 60 * 1000);
  return () => clearInterval(id);
}, [projects]);
```

### 通知权限
```typescript
if (Notification.permission === 'default') {
  await Notification.requestPermission();
}
if (Notification.permission === 'granted') {
  new Notification('Git 更新', { body: `${name} 有 ${count} 个更新` });
}
```

---

## 边界情况处理

1. **非 Git 仓库** → 不显示 Git UI
2. **Git 未安装** → 禁用 Git 功能，显示提示
3. **有未提交更改** → Pull 按钮禁用，批量操作跳过
4. **网络失败** → 显示错误，提供重试
5. **无远程配置** → 不显示 ahead/behind
6. **终端达到上限** → + 按钮禁用
7. **进程清理** → 关闭终端时确保 kill 子进程

import { invoke } from '@tauri-apps/api/tauri';
import type {
  Workspace,
  WorkspaceRef,
  ProjectInfo,
  PortChange,
  ProcessInfo,
  TerminalSession,
  GitStatus,
  GitPullResult,
} from '../types';

// Workspace APIs
export async function createWorkspace(name: string, folders: string[]): Promise<Workspace> {
  return invoke('create_workspace', { name, folders });
}

export async function loadWorkspace(workspacePath: string): Promise<Workspace> {
  return invoke('load_workspace', { workspacePath });
}

export async function scanWorkspaceProjects(folders: string[]): Promise<ProjectInfo[]> {
  return invoke('scan_workspace_projects', { folders });
}

export async function saveWorkspace(workspace: Workspace): Promise<void> {
  return invoke('save_workspace', { workspace });
}

export async function addWorkspaceFolder(workspace: Workspace, folderPath: string): Promise<Workspace> {
  return invoke('add_workspace_folder', { workspace, folderPath });
}

export async function removeWorkspaceFolder(workspace: Workspace, folderPath: string): Promise<Workspace> {
  return invoke('remove_workspace_folder', { workspace, folderPath });
}

export async function deleteWorkspace(workspaceId: string, rootPath: string): Promise<void> {
  return invoke('delete_workspace', { workspaceId, rootPath });
}

export async function getWorkspaceList(): Promise<WorkspaceRef[]> {
  return invoke('get_workspace_list');
}

export async function updateProjectEnabled(
  workspace: Workspace,
  projectId: string,
  enabled: boolean
): Promise<Workspace> {
  return invoke('update_project_enabled', { workspace, projectId, enabled });
}

// Project APIs
export async function getProjectDetails(projectPath: string): Promise<ProjectInfo> {
  return invoke('get_project_details', { projectPath });
}

export async function rescanProject(projectPath: string): Promise<ProjectInfo> {
  return invoke('rescan_project', { projectPath });
}

export async function isZebrasProject(projectPath: string): Promise<boolean> {
  return invoke('is_zebras_project', { projectPath });
}

// Port APIs
export async function checkPortAvailable(port: number): Promise<boolean> {
  return invoke('check_port_available', { port });
}

export async function resolvePortConflicts(
  currentWorkspaceId: string,
  projects: ProjectInfo[],
  portRangeStart: number,
  portRangeEnd: number
): Promise<[ProjectInfo[], PortChange[]]> {
  return invoke('resolve_port_conflicts', {
    currentWorkspaceId,
    projects,
    portRangeStart,
    portRangeEnd,
  });
}

// Process APIs
export async function startProject(
  projectId: string,
  projectName: string,
  projectPath: string
): Promise<ProcessInfo> {
  return invoke('start_project', { projectId, projectName, projectPath });
}

export async function stopProject(processId: string): Promise<void> {
  return invoke('stop_project', { processId });
}

export async function getRunningProcesses(): Promise<ProcessInfo[]> {
  return invoke('get_running_processes');
}

export async function stopAllProjects(): Promise<void> {
  return invoke('stop_all_projects');
}

export async function startAllProjects(workspace: Workspace): Promise<ProcessInfo[]> {
  return invoke('start_all_projects', { workspace });
}

export async function runProjectTask(
  projectId: string,
  projectName: string,
  projectPath: string,
  task: 'npm_install' | 'pnpm_install' | 'npm_deploy'
): Promise<void> {
  return invoke('run_project_task', { projectId, projectName, projectPath, task });
}

// Terminal APIs
export async function createTerminalSession(projectId: string): Promise<TerminalSession> {
  return invoke('create_terminal_session', { projectId });
}

export async function getTerminalSessions(projectId: string): Promise<TerminalSession[]> {
  return invoke('get_terminal_sessions', { projectId });
}

export async function runTerminalCommand(
  sessionId: string,
  projectPath: string,
  command: string
): Promise<void> {
  return invoke('run_terminal_command', { sessionId, projectPath, command });
}

export async function killTerminalSession(sessionId: string): Promise<void> {
  return invoke('kill_terminal_session', { sessionId });
}

export async function closeTerminalSession(sessionId: string): Promise<void> {
  return invoke('close_terminal_session', { sessionId });
}

// Git APIs
export async function isGitRepo(path: string): Promise<boolean> {
  return invoke('is_git_repo', { path });
}

export async function getGitStatus(path: string): Promise<GitStatus> {
  return invoke('get_git_status', { path });
}

export async function gitFetch(path: string): Promise<GitStatus> {
  return invoke('git_fetch', { path });
}

export async function gitPull(path: string): Promise<GitPullResult> {
  return invoke('git_pull', { path });
}

// Debug APIs
export async function updateDebugConfig(
  projectPath: string,
  projectVersion: string,
  debugMap: Record<string, string>
): Promise<void> {
  return invoke('update_debug_config', {
    projectPath,
    projectVersion,
    debugMap,
  });
}

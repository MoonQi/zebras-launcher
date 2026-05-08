export interface ProjectInfo {
  id: string;
  path: string;
  version: ZebrasVersion;
  source_type: ProjectSourceType;
  repo_role?: RepoRole;
  provision_status?: ProvisionStatus;
  platform: string;
  type: string;
  name: string;
  domain?: string;
  port: number;
  framework?: string;
  is_valid: boolean;
  last_scanned: string;
  error?: string;
  debug?: Record<string, string>; // 调试依赖配置，key: 项目名, value: URL
  enabled?: boolean; // 是否在"全部启动"时启动此项目，默认为 true
  runnable: boolean;
}

export type ZebrasVersion = 'v2' | 'v3' | 'managed';
export type ProjectSourceType = 'zebras' | 'managed_project';
export type RepoRole = 'frontend_app' | 'backend_service' | 'frontend_package';
export type ProvisionStatus = 'pending' | 'provisioning' | 'ready' | 'degraded';

export interface PortChange {
  project_name: string;
  old_port: number;
  new_port: number;
}

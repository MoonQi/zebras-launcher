import type { ProvisionStatus } from "./project";
import { ProjectInfo } from "./project";

export interface Workspace {
  id: string;
  name: string;
  root_path: string;
  source_type: WorkspaceSourceType;
  provision_status?: ProvisionStatus;
  folders: string[];  // 包含的多个代码文件夹路径
  created_at: string;
  last_modified: string;
  projects: ProjectInfo[];
  settings: WorkspaceSettings;
}

export interface WorkspaceSettings {
  auto_start_all: boolean;
  port_strategy: PortStrategy;
  port_range_start: number;
  port_range_end: number;
}

export type PortStrategy = 'sequential' | 'fixed';
export type WorkspaceSourceType = 'folder_scan' | 'managed_project';

import { ProjectInfo } from "./project";

export interface Workspace {
  id: string;
  name: string;
  root_path: string;
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

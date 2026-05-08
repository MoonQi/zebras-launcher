import type { WorkspaceSourceType } from './workspace';

export interface WorkspaceRef {
  id: string;
  name: string;
  config_path: string;
  source_type: WorkspaceSourceType;
  last_opened: string | null;
}

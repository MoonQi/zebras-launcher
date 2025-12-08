export interface ProjectInfo {
  id: string;
  path: string;
  version: ZebrasVersion;
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
}

export type ZebrasVersion = 'v2' | 'v3';

export interface PortChange {
  project_name: string;
  old_port: number;
  new_port: number;
}

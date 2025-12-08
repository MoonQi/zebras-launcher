export interface ProcessInfo {
  process_id: string;
  project_id: string;
  project_name: string;
  status: ProcessStatus;
  started_at: string;
  pid: number | null;
}

export type ProcessStatus = "starting" | "running" | "stopped" | "crashed";

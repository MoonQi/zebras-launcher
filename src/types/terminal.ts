export type TerminalStatus = 'idle' | 'running' | 'completed' | 'error';

export interface TerminalSession {
  session_id: string;
  project_id: string;
  command: string | null;
  status: TerminalStatus;
  pid: number | null;
}

export interface TerminalLogMessage {
  session_id: string;
  project_id: string;
  message: string;
  stream: 'stdout' | 'stderr';
}


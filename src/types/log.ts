export interface LogMessage {
  process_id: string;
  session_id?: string | null;
  project_id: string;
  project_name: string;
  message: string;
  stream: 'stdout' | 'stderr';
}

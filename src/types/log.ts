export interface LogMessage {
  process_id: string;
  project_name: string;
  message: string;
  stream: 'stdout' | 'stderr';
}

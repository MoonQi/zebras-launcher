export interface GitStatus {
  branch: string | null;
  has_remote: boolean;
  uncommitted_count: number;
  ahead_count: number;
  behind_count: number;
}

export interface GitPullResult {
  success: boolean;
  message: string;
  status: GitStatus;
}


export interface GitStatus {
  branch: string | null;
  has_remote: boolean;
  uncommitted_count: number;
  ahead_count: number;
  behind_count: number;
}

export interface GitBranch {
  name: string;
  is_remote: boolean;
  is_current: boolean;
  upstream: string | null;
}

export interface GitPullResult {
  success: boolean;
  message: string;
  status: GitStatus;
}

export interface GitSwitchResult {
  message: string;
  status: GitStatus;
}

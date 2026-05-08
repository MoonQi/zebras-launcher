import type { ProvisionStatus, RepoRole } from './project';

export interface ManagedRepoInput {
  id: string;
  display_name: string;
  role: RepoRole;
  git_url: string;
  branch?: string;
}

export interface ManagedFrontendLinkInput {
  provider_repo_id: string;
  consumer_repo_id: string;
}

export interface CreateProjectInstanceInput {
  project_name: string;
  root_path: string;
  repos: ManagedRepoInput[];
  frontend_links: ManagedFrontendLinkInput[];
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
}

export interface ManagedProjectManifestRepo {
  id: string;
  display_name: string;
  role: RepoRole;
  git_url: string;
  branch?: string;
  mount_path: string;
  status: ProvisionStatus;
  last_error?: string;
}

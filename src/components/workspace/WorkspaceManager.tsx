import { useEffect, useMemo, useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { validateProjectInstance } from '../../services/tauri';
import type {
  CreateProjectInstanceInput,
  ManagedFrontendLinkInput,
  ManagedRepoInput,
  RepoRole,
  Workspace,
} from '../../types';

type CreateMode = 'workspace' | 'managed' | null;

function deriveRepoName(gitUrl: string): string {
  const trimmed = gitUrl.trim().replace(/\/+$/, '');
  if (!trimmed) return '';
  const candidate = trimmed.split(/[/:]/).filter(Boolean).pop() ?? '';
  return candidate.replace(/\.git$/i, '');
}

interface WorkspaceManagerProps {
  workspace: Workspace | null;
  onCreateWorkspace: (name: string) => Promise<void> | void;
  onCreateManagedProject: (input: CreateProjectInstanceInput) => Promise<void> | void;
  onDeleteWorkspace: () => void;
  onRescan: () => void;
  onResolveConflicts: () => void;
  onOpenDependencyGraph: () => void;
  onAddFolder: () => void;
  onRemoveFolder: (folderPath: string) => void;
  onStartAll: () => void;
  onStopAll: () => void;
  onRepairManagedProject: () => void;
  onRebuildManagedLinks: () => void;
  loading: boolean;
  allProjectsRunning: boolean;
}

function emptyRepo(role: RepoRole = 'frontend_app'): ManagedRepoInput {
  return {
    id: '',
    display_name: '',
    role,
    git_url: '',
  };
}

function emptyLink(): ManagedFrontendLinkInput {
  return {
    provider_repo_id: '',
    consumer_repo_id: '',
  };
}

export function WorkspaceManager({
  workspace,
  onCreateWorkspace,
  onCreateManagedProject,
  onDeleteWorkspace,
  onRescan,
  onResolveConflicts,
  onOpenDependencyGraph,
  onAddFolder,
  onRemoveFolder,
  onStartAll,
  onStopAll,
  onRepairManagedProject,
  onRebuildManagedLinks,
  loading,
  allProjectsRunning,
}: WorkspaceManagerProps) {
  const [workspaceName, setWorkspaceName] = useState('');
  const [createMode, setCreateMode] = useState<CreateMode>(workspace ? null : null);
  const [projectName, setProjectName] = useState('');
  const [projectRootPath, setProjectRootPath] = useState('');
  const [repos, setRepos] = useState<ManagedRepoInput[]>([
    emptyRepo('frontend_app'),
    emptyRepo('backend_service'),
  ]);
  const [frontendLinks, setFrontendLinks] = useState<ManagedFrontendLinkInput[]>([]);
  const [validationErrors, setValidationErrors] = useState<string[]>([]);
  const [validating, setValidating] = useState(false);

  useEffect(() => {
    if (workspace) {
      setCreateMode(null);
    }
  }, [workspace]);

  const isManagedProject = workspace?.source_type === 'managed_project';
  const runnableProjects = useMemo(
    () => workspace?.projects.filter((project) => project.is_valid && project.runnable) ?? [],
    [workspace]
  );
  const providerRepos = useMemo(
    () =>
      repos
        .map((repo) => ({
          ...repo,
          effectiveId: repo.id.trim() || deriveRepoName(repo.git_url),
          effectiveDisplayName: repo.display_name.trim() || deriveRepoName(repo.git_url),
        }))
        .filter((repo) => repo.role === 'frontend_package' && repo.effectiveId),
    [repos]
  );
  const consumerRepos = useMemo(
    () =>
      repos
        .map((repo) => ({
          ...repo,
          effectiveId: repo.id.trim() || deriveRepoName(repo.git_url),
          effectiveDisplayName: repo.display_name.trim() || deriveRepoName(repo.git_url),
        }))
        .filter((repo) => repo.role === 'frontend_app' && repo.effectiveId),
    [repos]
  );

  const resetCreateState = () => {
    setWorkspaceName('');
    setProjectName('');
    setProjectRootPath('');
    setRepos([emptyRepo('frontend_app'), emptyRepo('backend_service')]);
    setFrontendLinks([]);
    setValidationErrors([]);
    setCreateMode(null);
  };

  const handleCreateWorkspace = async () => {
    if (!workspaceName.trim()) return;
    await onCreateWorkspace(workspaceName.trim());
    resetCreateState();
  };

  const buildManagedInput = (): CreateProjectInstanceInput => ({
    project_name: projectName.trim(),
    root_path: projectRootPath.trim(),
    repos: repos.map((repo) => ({
      ...repo,
      id: repo.id.trim() || deriveRepoName(repo.git_url),
      display_name: repo.display_name.trim() || deriveRepoName(repo.git_url),
      git_url: repo.git_url.trim(),
    })),
    frontend_links: frontendLinks
      .filter((link) => link.provider_repo_id && link.consumer_repo_id)
      .map((link) => ({ ...link })),
  });

  const handleBrowseRoot = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择项目实例根目录（需为空目录）',
    });
    if (selected && !Array.isArray(selected)) {
      setProjectRootPath(selected);
    }
  };

  const handleValidateManagedProject = async () => {
    setValidating(true);
    try {
      const result = await validateProjectInstance(buildManagedInput());
      setValidationErrors(result.errors);
      return result.valid;
    } finally {
      setValidating(false);
    }
  };

  const handleCreateManagedProject = async () => {
    const valid = await handleValidateManagedProject();
    if (!valid) return;
    await onCreateManagedProject(buildManagedInput());
    resetCreateState();
  };

  const renderCreateWorkspaceForm = () => (
    <div className="card max-w-lg mx-auto">
      <h2 className="mb-md">创建目录工作区</h2>
      <div className="mb-md">
        <label className="block mb-sm text-secondary text-sm">工作区名称</label>
        <input
          type="text"
          className="input"
          value={workspaceName}
          onChange={(e) => setWorkspaceName(e.target.value)}
          placeholder="例如：我的项目集"
          onKeyDown={(e) => {
            if (e.key === 'Enter') void handleCreateWorkspace();
          }}
        />
      </div>
      <div className="flex gap-md">
        <button
          onClick={() => void handleCreateWorkspace()}
          disabled={!workspaceName.trim() || loading}
          className="btn btn-primary flex-1"
        >
          {loading ? '创建中...' : '选择目录并创建'}
        </button>
        <button onClick={resetCreateState} className="btn btn-secondary">
          取消
        </button>
      </div>
      <p className="mt-md text-sm text-secondary">
        点击后会打开文件选择对话框。可以按住 Ctrl/Command 键多选多个文件夹。
      </p>
    </div>
  );

  const renderManagedProjectForm = () => (
    <div className="card max-w-4xl mx-auto">
      <div className="flex justify-between items-start gap-md mb-lg">
        <div>
          <h2 className="mb-sm">创建项目实例</h2>
          <p className="text-secondary m-0">
            输入一个业务项目的前后端仓库清单，应用会自动装配本地开发目录并生成 `.zebras-project.json`。
          </p>
        </div>
        <button onClick={resetCreateState} className="btn btn-secondary">
          取消
        </button>
      </div>

      <div className="grid gap-md" style={{ gridTemplateColumns: '1fr 1.2fr' }}>
        <div>
          <label className="block mb-sm text-secondary text-sm">项目名称</label>
          <input
            type="text"
            className="input"
            value={projectName}
            onChange={(e) => setProjectName(e.target.value)}
            placeholder="例如：Acme 项目"
          />
        </div>
        <div>
          <label className="block mb-sm text-secondary text-sm">本地根目录</label>
          <div className="flex gap-sm">
            <input
              type="text"
              className="input"
              value={projectRootPath}
              onChange={(e) => setProjectRootPath(e.target.value)}
              placeholder="/path/to/project-instance"
            />
            <button onClick={() => void handleBrowseRoot()} className="btn btn-secondary">
              选择
            </button>
          </div>
        </div>
      </div>

      <div className="mt-lg">
        <div className="flex justify-between items-center mb-sm">
          <h3 className="m-0">仓库清单</h3>
          <div className="flex gap-sm">
            <button onClick={() => setRepos((prev) => [...prev, emptyRepo('frontend_app')])} className="btn btn-secondary btn-sm">
              + 前端应用
            </button>
            <button onClick={() => setRepos((prev) => [...prev, emptyRepo('backend_service')])} className="btn btn-secondary btn-sm">
              + 后端服务
            </button>
            <button onClick={() => setRepos((prev) => [...prev, emptyRepo('frontend_package')])} className="btn btn-secondary btn-sm">
              + 前端共享包
            </button>
          </div>
        </div>

        <div className="grid gap-sm">
          {repos.map((repo, index) => (
            <div
              key={`${index}-${repo.role}`}
              className="grid gap-sm p-sm rounded border border-border"
              style={{ gridTemplateColumns: '1fr 1fr 0.9fr 1.2fr auto', alignItems: 'end' }}
            >
              <div>
                <label className="block mb-xs text-xs text-secondary">ID / 目录名</label>
                  <input
                    type="text"
                    className="input"
                    value={repo.id}
                  onChange={(e) =>
                    setRepos((prev) =>
                      prev.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, id: e.target.value } : item
                      )
                    )
                  }
                  placeholder="留空则使用 URL 最后名称"
                />
              </div>
              <div>
                <label className="block mb-xs text-xs text-secondary">显示名</label>
                <input
                  type="text"
                  className="input"
                  value={repo.display_name}
                  onChange={(e) =>
                    setRepos((prev) =>
                      prev.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, display_name: e.target.value } : item
                      )
                    )
                  }
                  placeholder="留空则使用 URL 最后名称"
                />
              </div>
              <div>
                <label className="block mb-xs text-xs text-secondary">角色</label>
                <select
                  className="select"
                  value={repo.role}
                  onChange={(e) =>
                    setRepos((prev) =>
                      prev.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, role: e.target.value as RepoRole } : item
                      )
                    )
                  }
                >
                  <option value="frontend_app">frontend_app</option>
                  <option value="backend_service">backend_service</option>
                  <option value="frontend_package">frontend_package</option>
                </select>
              </div>
              <div>
                <label className="block mb-xs text-xs text-secondary">Git URL</label>
                <input
                  type="text"
                  className="input"
                  value={repo.git_url}
                  onChange={(e) =>
                    setRepos((prev) =>
                      prev.map((item, itemIndex) => {
                        if (itemIndex !== index) return item;
                        const nextUrl = e.target.value;
                        const derivedName = deriveRepoName(nextUrl);
                        return {
                          ...item,
                          git_url: nextUrl,
                          id: item.id.trim() ? item.id : derivedName,
                          display_name: item.display_name.trim()
                            ? item.display_name
                            : derivedName,
                        };
                      })
                    )
                  }
                  placeholder="https://.../_git/report_saas_zebras_main"
                />
                <div className="text-xs text-secondary mt-xs">
                  默认按远端默认分支 clone。目录名和显示名留空时，会自动使用 URL 最后名称。
                </div>
              </div>
              <button
                onClick={() =>
                  setRepos((prev) => prev.filter((_, itemIndex) => itemIndex !== index))
                }
                className="btn btn-ghost btn-sm text-danger"
                disabled={repos.length <= 1}
              >
                删除
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-lg">
        <div className="flex justify-between items-center mb-sm">
          <h3 className="m-0">前端 Links</h3>
          <button onClick={() => setFrontendLinks((prev) => [...prev, emptyLink()])} className="btn btn-secondary btn-sm">
            + 添加 Link
          </button>
        </div>
        {frontendLinks.length === 0 ? (
          <div className="text-sm text-secondary p-sm rounded border border-border">
            当前未配置前端共享包 link。只有项目内存在独立 `frontend_package` 仓库时才需要这里。
          </div>
        ) : (
          <div className="grid gap-sm">
            {frontendLinks.map((link, index) => (
              <div
                key={`${index}-${link.provider_repo_id}-${link.consumer_repo_id}`}
                className="grid gap-sm p-sm rounded border border-border"
                style={{ gridTemplateColumns: '1fr 1fr auto', alignItems: 'end' }}
              >
                <div>
                  <label className="block mb-xs text-xs text-secondary">Provider</label>
                  <select
                    className="select"
                    value={link.provider_repo_id}
                    onChange={(e) =>
                      setFrontendLinks((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index
                            ? { ...item, provider_repo_id: e.target.value }
                            : item
                        )
                      )
                    }
                  >
                    <option value="">选择 frontend_package</option>
                    {providerRepos.map((repo) => (
                      <option key={repo.effectiveId} value={repo.effectiveId}>
                        {repo.effectiveDisplayName || repo.effectiveId}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block mb-xs text-xs text-secondary">Consumer</label>
                  <select
                    className="select"
                    value={link.consumer_repo_id}
                    onChange={(e) =>
                      setFrontendLinks((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index
                            ? { ...item, consumer_repo_id: e.target.value }
                            : item
                        )
                      )
                    }
                  >
                    <option value="">选择 frontend_app</option>
                    {consumerRepos.map((repo) => (
                      <option key={repo.effectiveId} value={repo.effectiveId}>
                        {repo.effectiveDisplayName || repo.effectiveId}
                      </option>
                    ))}
                  </select>
                </div>
                <button
                  onClick={() =>
                    setFrontendLinks((prev) => prev.filter((_, itemIndex) => itemIndex !== index))
                  }
                  className="btn btn-ghost btn-sm text-danger"
                >
                  删除
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {validationErrors.length > 0 && (
        <div
          className="mt-lg p-md rounded"
          style={{ backgroundColor: 'rgba(239, 68, 68, 0.08)', border: '1px solid rgba(239, 68, 68, 0.2)' }}
        >
          <div className="text-danger font-semibold mb-sm">校验失败</div>
          <ul className="m-0 pl-lg text-sm text-danger">
            {validationErrors.map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex gap-sm mt-lg">
        <button
          onClick={() => void handleValidateManagedProject()}
          disabled={loading || validating}
          className="btn btn-secondary"
        >
          {validating ? '校验中...' : '先校验'}
        </button>
        <button
          onClick={() => void handleCreateManagedProject()}
          disabled={loading || validating}
          className="btn btn-primary"
        >
          {loading ? '创建中...' : '创建项目实例'}
        </button>
      </div>
    </div>
  );

  if (!workspace && !createMode) {
    return (
      <div className="text-center py-lg">
        <h2 className="mb-md">欢迎使用 Zebras Launcher</h2>
        <p className="mb-md text-secondary">创建或选择一个工作区以开始管理您的项目。</p>
        <div className="flex justify-center gap-md">
          <button onClick={() => setCreateMode('workspace')} className="btn btn-primary">
            创建目录工作区
          </button>
          <button onClick={() => setCreateMode('managed')} className="btn btn-secondary">
            创建项目实例
          </button>
        </div>
      </div>
    );
  }

  if (createMode === 'workspace') {
    return renderCreateWorkspaceForm();
  }

  if (createMode === 'managed') {
    return renderManagedProjectForm();
  }

  if (!workspace) {
    return null;
  }

  return (
    <div className="card mb-lg">
      <div
        className="flex justify-between items-start mb-lg pb-md border-b border-border"
        style={{ borderBottom: '1px solid var(--color-border)' }}
      >
        <div>
          <div className="flex items-center gap-md mb-xs">
            <h2 className="m-0">{workspace.name}</h2>
            <div className="flex gap-xs flex-wrap">
              <span
                className="badge bg-surface-hover text-secondary"
                style={{ backgroundColor: 'var(--color-surface-hover)' }}
              >
                {workspace.projects.length} 个仓库
              </span>
              <span
                className="badge"
                style={{
                  backgroundColor:
                    workspace.source_type === 'managed_project'
                      ? 'rgba(59, 130, 246, 0.1)'
                      : 'var(--color-surface-hover)',
                  color:
                    workspace.source_type === 'managed_project'
                      ? 'var(--color-primary)'
                      : 'var(--color-text-secondary)',
                }}
              >
                {workspace.source_type === 'managed_project' ? '项目实例' : `${workspace.folders.length} 个文件夹`}
              </span>
              {workspace.provision_status && (
                <span
                  className="badge"
                  style={{
                    backgroundColor:
                      workspace.provision_status === 'degraded'
                        ? 'rgba(239, 68, 68, 0.1)'
                        : workspace.provision_status === 'ready'
                          ? 'rgba(16, 185, 129, 0.1)'
                          : 'rgba(245, 158, 11, 0.1)',
                    color:
                      workspace.provision_status === 'degraded'
                        ? 'var(--color-danger)'
                        : workspace.provision_status === 'ready'
                          ? 'var(--color-success)'
                          : 'var(--color-warning)',
                  }}
                >
                  {workspace.provision_status}
                </span>
              )}
            </div>
          </div>
          <p className="m-0 text-sm text-muted font-mono">{workspace.root_path}</p>
        </div>

        <div className="flex gap-sm">
          <button
            onClick={() => setCreateMode('workspace')}
            className="btn btn-ghost btn-sm text-secondary"
          >
            新建目录工作区
          </button>
          <button
            onClick={() => setCreateMode('managed')}
            className="btn btn-ghost btn-sm text-secondary"
          >
            新建项目实例
          </button>
          <button
            onClick={() => {
              if (confirm(`确定要删除工作区"${workspace.name}"吗？\n\n这将删除工作区配置文件，但不会删除项目文件。`)) {
                onDeleteWorkspace();
              }
            }}
            disabled={loading}
            className="btn btn-ghost btn-sm text-danger hover:bg-danger hover:text-white"
            style={{ transition: 'all 0.2s' }}
          >
            删除工作区
          </button>
        </div>
      </div>

      {isManagedProject ? (
        <div className="grid gap-md">
          <div className="flex flex-wrap gap-sm">
            <button onClick={onRescan} disabled={loading} className="btn btn-secondary">
              重新加载实例
            </button>
            <button onClick={onRepairManagedProject} disabled={loading} className="btn btn-secondary">
              修复项目
            </button>
            <button onClick={onRebuildManagedLinks} disabled={loading} className="btn btn-secondary">
              重建 Links
            </button>
          </div>

          <div
            className="rounded p-md"
            style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', border: '1px solid var(--color-border)' }}
          >
            <div className="text-sm font-semibold mb-sm">实例说明</div>
            <div className="text-sm text-secondary">
              该工作区由 `.zebras-project.json` 驱动，仓库列表来自 manifest，而不是目录扫描。前端仓库若能识别到 Zebras 配置，会在项目卡片上支持直接启动；批量启动仍默认关闭。
            </div>
          </div>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-md mb-lg">
            <div className="flex gap-sm">
              <button
                onClick={onStartAll}
                disabled={loading || allProjectsRunning || runnableProjects.length === 0}
                className="btn btn-success"
                style={{
                  minWidth: '120px',
                  opacity: allProjectsRunning || runnableProjects.length === 0 ? 0.5 : 1,
                  cursor:
                    loading || allProjectsRunning || runnableProjects.length === 0
                      ? 'not-allowed'
                      : 'pointer',
                }}
                title={allProjectsRunning ? '所有有效项目已在运行' : '启动所有有效项目'}
              >
                全部启动
              </button>
              <button
                onClick={onStopAll}
                disabled={loading}
                className="btn btn-danger"
                style={{ minWidth: '120px' }}
              >
                全部停止
              </button>
            </div>

            <div
              className="w-px h-8 bg-border hidden sm:block"
              style={{ width: '1px', height: '32px', backgroundColor: 'var(--color-border)' }}
            />

            <div className="flex gap-sm flex-1">
              <button onClick={onRescan} disabled={loading} className="btn btn-secondary">
                重新扫描
              </button>

              <button onClick={onResolveConflicts} disabled={loading} className="btn btn-secondary">
                解决端口冲突
              </button>

              <button onClick={onOpenDependencyGraph} disabled={loading} className="btn btn-secondary">
                依赖图
              </button>
            </div>
          </div>

          <div className="bg-bg rounded p-[10px]" style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)' }}>
            <div className="flex items-center justify-between mb-md">
              <h3 className="m-0 text-sm font-semibold text-secondary uppercase tracking-wider">
                监控文件夹
              </h3>
              <button onClick={onAddFolder} disabled={loading} className="btn btn-primary btn-sm">
                + 添加文件夹
              </button>
            </div>

            <div className="grid gap-sm" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))' }}>
              {workspace.folders.map((folder, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-sm rounded border border-border bg-surface"
                  style={{
                    borderColor: 'var(--color-border)',
                    borderWidth: '1px',
                    borderStyle: 'solid',
                  }}
                >
                  <div className="flex items-center gap-sm overflow-hidden flex-1 mr-sm">
                    <span className="text-lg opacity-50">📁</span>
                    <div className="overflow-hidden">
                      <div className="text-sm text-main font-medium overflow-hidden text-ellipsis whitespace-nowrap">
                        {folder.split(/[/\\]/).pop()}
                      </div>
                      <div className="text-xs text-muted overflow-hidden text-ellipsis whitespace-nowrap" title={folder}>
                        {folder}
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => onRemoveFolder(folder)}
                    disabled={loading || workspace.folders.length === 1}
                    className={`btn btn-ghost btn-sm ${
                      workspace.folders.length === 1
                        ? 'text-muted cursor-not-allowed'
                        : 'text-danger hover:bg-danger hover:text-white'
                    }`}
                    title={workspace.folders.length === 1 ? '至少保留一个文件夹' : '移除此文件夹'}
                    style={{ padding: '4px 8px' }}
                  >
                    移除
                  </button>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

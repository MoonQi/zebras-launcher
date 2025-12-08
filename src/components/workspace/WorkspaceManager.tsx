import { useState, useEffect } from 'react';
import type { Workspace } from '../../types';

interface WorkspaceManagerProps {
  workspace: Workspace | null;
  onCreateWorkspace: (name: string) => void;
  onDeleteWorkspace: () => void;
  onRescan: () => void;
  onResolveConflicts: () => void;
  onAddFolder: () => void;
  onRemoveFolder: (folderPath: string) => void;
  onStartAll: () => void;
  onStopAll: () => void;
  loading: boolean;
  allProjectsRunning: boolean;
}

export function WorkspaceManager({
  workspace,
  onCreateWorkspace,
  onDeleteWorkspace,
  onRescan,
  onResolveConflicts,
  onAddFolder,
  onRemoveFolder,
  onStartAll,
  onStopAll,
  loading,
  allProjectsRunning,
}: WorkspaceManagerProps) {
  const [workspaceName, setWorkspaceName] = useState('');
  const [showCreateForm, setShowCreateForm] = useState(!workspace);

  // 当workspace变化时，自动关闭创建表单
  useEffect(() => {
    if (workspace) {
      setShowCreateForm(false);
    }
  }, [workspace]);

  const handleCreate = () => {
    if (!workspaceName.trim()) return;
    onCreateWorkspace(workspaceName.trim());
    setWorkspaceName('');
    setShowCreateForm(false);
  };

  const renderCreateWorkspaceForm = () => (
    <div className="card max-w-lg mx-auto">
      <h2 className="mb-md">创建工作区</h2>
      <div className="mb-md">
        <label className="block mb-sm text-secondary text-sm">工作区名称</label>
        <input
          type="text"
          className="input"
          value={workspaceName}
          onChange={(e) => setWorkspaceName(e.target.value)}
          placeholder="例如：我的项目集"
          onKeyPress={(e) => {
            if (e.key === 'Enter') handleCreate();
          }}
        />
      </div>
      <div className="flex gap-md">
        <button
          onClick={handleCreate}
          disabled={!workspaceName.trim() || loading}
          className="btn btn-primary flex-1"
        >
          {loading ? '创建中...' : '选择目录并创建'}
        </button>
        <button onClick={() => setShowCreateForm(false)} className="btn btn-secondary">
          取消
        </button>
      </div>
      <p className="mt-md text-sm text-secondary">
        点击"选择目录并创建"后，会打开文件选择对话框。<strong>可以按住 Ctrl/Command 键多选多个文件夹</strong>。
      </p>
    </div>
  );

  if (!workspace && !showCreateForm) {
    return (
      <div className="text-center py-lg">
        <h2 className="mb-md">欢迎使用 Zebras Launcher</h2>
        <p className="mb-md text-secondary">创建或选择一个工作区以开始管理您的项目。</p>
        <button onClick={() => setShowCreateForm(true)} className="btn btn-primary">
          创建工作区
        </button>
      </div>
    );
  }

  if (showCreateForm) {
    return renderCreateWorkspaceForm();
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
              <div className="flex gap-xs">
                <span
                  className="badge bg-surface-hover text-secondary"
                  style={{ backgroundColor: 'var(--color-surface-hover)' }}
                >
                  {workspace.projects.length} 个项目
                </span>
                <span
                  className="badge bg-surface-hover text-secondary"
                  style={{ backgroundColor: 'var(--color-surface-hover)' }}
                >
                  {workspace.folders.length} 个文件夹
                </span>
              </div>
            </div>
            <p className="m-0 text-sm text-muted font-mono">{workspace.root_path}</p>
          </div>

          <div className="flex gap-sm">
            <button
              onClick={() => {
                setWorkspaceName('');
                setShowCreateForm(true);
              }}
              className="btn btn-ghost btn-sm text-secondary"
            >
              新建/切换工作区
            </button>
            <button
              onClick={() => {
                if (confirm(`确定要删除工作区"${workspace.name}"吗？\n\n这将删除工作区配置文件，但不会删除项目文件。`)) {
                  onDeleteWorkspace();
                }
              }}
              disabled={loading}
              className="btn btn-ghost btn-sm text-danger hover:bg-danger hover:text-white"
              style={{
                transition: 'all 0.2s',
              }}
            >
              删除工作区
            </button>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-md mb-lg">
          <div className="flex gap-sm">
            <button
              onClick={onStartAll}
              disabled={loading || allProjectsRunning}
              className="btn btn-success"
              style={{
                minWidth: '120px',
                opacity: allProjectsRunning ? 0.5 : 1,
                cursor: loading || allProjectsRunning ? 'not-allowed' : 'pointer',
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
    </div>
  );
}

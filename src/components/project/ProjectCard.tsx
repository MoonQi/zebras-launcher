import { useState, useEffect, useMemo, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/shell';
import type { ProjectInfo, ProcessInfo, LogMessage, Workspace, GitBranch, GitPullResult, GitStatus } from '../../types';
import {
  gitSwitchBranch,
  listGitBranches,
  startProject,
  stopProject,
  updateDebugConfig,
  updateProjectEnabled,
} from '../../services/tauri';
import { TerminalPanel } from './TerminalPanel';

interface ProjectCardProps {
  project: ProjectInfo;
  processInfo?: ProcessInfo;
  onProcessStart: (projectId: string, processInfo: ProcessInfo) => void;
  onProcessStop: (projectId: string) => void;
  allProjects: ProjectInfo[];
  workspace: Workspace;
  onWorkspaceUpdate: (workspace: Workspace) => void;
  gitStatus?: GitStatus | null;
  gitBusy?: { fetching: boolean; pulling: boolean };
  gitDisabledReason?: string | null;
  onGitFetch: (project: ProjectInfo) => Promise<void>;
  onGitPull: (project: ProjectInfo) => Promise<GitPullResult | null>;
  onGitRefresh: (project: ProjectInfo) => Promise<void>;
}

export function ProjectCard({
  project,
  processInfo,
  onProcessStart,
  onProcessStop,
  allProjects,
  workspace,
  onWorkspaceUpdate,
  gitStatus,
  gitBusy,
  gitDisabledReason,
  onGitFetch,
  onGitPull,
  onGitRefresh,
}: ProjectCardProps) {
  const [isStarting, setIsStarting] = useState(false);
  const [logs, setLogs] = useState<LogMessage[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [followLogs, setFollowLogs] = useState(true);
  const [showDebugConfig, setShowDebugConfig] = useState(false);
  const [showTerminal, setShowTerminal] = useState(false);
  const [debugConfig, setDebugConfig] = useState<Record<string, string>>(project.debug || {});
  const [selectedProject, setSelectedProject] = useState<string>('');
  const logContainerRef = useRef<HTMLDivElement>(null);
  const branchPickerRef = useRef<HTMLDivElement>(null);
  const [showBranchPicker, setShowBranchPicker] = useState(false);
  const [branchSearch, setBranchSearch] = useState('');
  const [branchListLoading, setBranchListLoading] = useState(false);
  const [branchSwitchingName, setBranchSwitchingName] = useState<string | null>(null);
  const [branchListError, setBranchListError] = useState<string | null>(null);
  const [branches, setBranches] = useState<GitBranch[]>([]);

  useEffect(() => {
    setDebugConfig(project.debug || {});
  }, [project.debug]);

  useEffect(() => {
    const unlisten = listen<LogMessage>('process_log', (event) => {
      const log = event.payload;
      if (log.project_id === project.id) {
        setLogs(prev => [...prev, log]);
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [project.id]);

  useEffect(() => {
    if (!showLogs || !followLogs) return;
    const container = logContainerRef.current;
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
    }
  }, [logs, showLogs, followLogs]);

  useEffect(() => {
    if (processInfo?.status === 'running') {
      setShowLogs(true);
      setFollowLogs(true);
    }
  }, [processInfo?.status]);

  useEffect(() => {
    if (!showBranchPicker) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (branchPickerRef.current && !branchPickerRef.current.contains(event.target as Node)) {
        setShowBranchPicker(false);
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    return () => document.removeEventListener('mousedown', handlePointerDown);
  }, [showBranchPicker]);

  useEffect(() => {
    setShowBranchPicker(false);
    setBranchSearch('');
    setBranchListError(null);
    setBranches([]);
  }, [project.id]);

  const handleStart = async () => {
    try {
      setIsStarting(true);
      setLogs([]);
      const info = await startProject(project.id, project.name, project.path);
      onProcessStart(project.id, info);
    } catch (err) {
      alert(`启动失败: ${err}`);
    } finally {
      setIsStarting(false);
    }
  };

  const handleStop = async () => {
    if (!processInfo) return;
    try {
      setIsStarting(true);
      await stopProject(processInfo.process_id);
      onProcessStop(project.id);
    } catch (err) {
      alert(`停止失败: ${err}`);
    } finally {
      setIsStarting(false);
    }
  };

  const handleRestart = async () => {
    if (!processInfo) return;
    try {
      setIsStarting(true);
      setLogs([]);
      await stopProject(processInfo.process_id);
      onProcessStop(project.id);
      const info = await startProject(project.id, project.name, project.path);
      onProcessStart(project.id, info);
      setShowLogs(true);
      setFollowLogs(true);
    } catch (err) {
      alert(`重启失败: ${err}`);
    } finally {
      setIsStarting(false);
    }
  };

  const handleAddDebugDep = async () => {
    if (!selectedProject) return;
    const targetProject = allProjects.find(p => p.name === selectedProject);
    if (!targetProject) return;

    const url = `http://localhost:${targetProject.port}`;
    const newDebugConfig = { ...debugConfig, [selectedProject]: url };

    try {
      await updateDebugConfig(project.path, project.version, newDebugConfig);
      setDebugConfig(newDebugConfig);
      setSelectedProject('');
    } catch (err) {
      alert(`添加调试依赖失败: ${err}`);
    }
  };

  const handleRemoveDebugDep = async (depName: string) => {
    const newDebugConfig = { ...debugConfig };
    delete newDebugConfig[depName];

    try {
      await updateDebugConfig(project.path, project.version, newDebugConfig);
      setDebugConfig(newDebugConfig);
    } catch (err) {
      alert(`移除调试依赖失败: ${err}`);
    }
  };

  const handleToggleEnabled = async () => {
    const newEnabled = !(project.enabled ?? true);
    try {
      const updatedWorkspace = await updateProjectEnabled(workspace, project.id, newEnabled);
      onWorkspaceUpdate(updatedWorkspace);
    } catch (err) {
      alert(`更新启用状态失败: ${err}`);
    }
  };

  const handleLogsScroll = () => {
    const container = logContainerRef.current;
    if (!container) return;
    const isNearBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 16;
    if (followLogs !== isNearBottom) {
      setFollowLogs(isNearBottom);
    }
  };

  const handleGitFetch = async () => {
    try {
      await onGitFetch(project);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      alert(`Git Fetch 失败: ${message}`);
    }
  };

  const handleGitPull = async () => {
    try {
      const result = await onGitPull(project);
      if (result && !result.success) {
        alert(result.message);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      alert(`Git Pull 失败: ${message}`);
    }
  };

  const loadBranchPicker = async () => {
    try {
      setBranchListLoading(true);
      setBranchListError(null);
      const nextBranches = await listGitBranches(project.path, true);
      setBranches(nextBranches);
      await onGitRefresh(project);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setBranchListError(message);
    } finally {
      setBranchListLoading(false);
    }
  };

  const handleToggleBranchPicker = async () => {
    const nextOpen = !showBranchPicker;
    setShowBranchPicker(nextOpen);
    if (!nextOpen) {
      return;
    }

    setBranchSearch('');
    await loadBranchPicker();
  };

  const handleSwitchBranch = async (branch: GitBranch) => {
    try {
      setBranchSwitchingName(branch.name);
      setBranchListError(null);
      const result = await gitSwitchBranch(project.path, branch.name, branch.is_remote);
      await onGitRefresh(project);
      setShowBranchPicker(false);
      setBranchSearch('');
      alert(result.message);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setBranchListError(message);
    } finally {
      setBranchSwitchingName(null);
    }
  };

  const isRunning = processInfo && processInfo.status === 'running';
  const isManagedProject = project.source_type === 'managed_project';
  const canControlProcess = project.is_valid && project.runnable;
  const canOpenByPort = project.port > 0;
  const availableProjects = allProjects.filter(
    p => p.name !== project.name && p.is_valid && !debugConfig[p.name]
  );
  const filteredBranches = useMemo(() => {
    const query = branchSearch.trim().toLowerCase();
    const nextBranches = query
      ? branches.filter((branch) => branch.name.toLowerCase().includes(query))
      : branches;

    const sorted = [...nextBranches].sort((left, right) => {
      if (left.is_current !== right.is_current) {
        return left.is_current ? -1 : 1;
      }
      return left.name.localeCompare(right.name);
    });

    return {
      local: sorted.filter((branch) => !branch.is_remote),
      remote: sorted.filter((branch) => branch.is_remote),
    };
  }, [branchSearch, branches]);

  const getVersionBadgeStyle = (version: string) => {
    if (version === 'managed') {
      return {
        backgroundColor: 'rgba(59, 130, 246, 0.12)',
        color: 'var(--color-primary)',
        border: '1px solid rgba(59, 130, 246, 0.25)',
      };
    }
    const isV3 = version === 'v3';
    return {
      backgroundColor: isV3 ? 'rgba(16, 185, 129, 0.1)' : 'rgba(59, 130, 246, 0.1)',
      color: isV3 ? 'var(--color-success)' : 'var(--color-primary)',
      border: `1px solid ${isV3 ? 'var(--color-success)' : 'var(--color-primary)'}`,
    };
  };

  const getTypeBadgeStyle = (type: string) => {
    let color = 'var(--color-text-secondary)';
    switch (type) {
      case 'frontend_app':
        color = 'var(--color-success)';
        break;
      case 'backend_service':
        color = 'var(--color-primary)';
        break;
      case 'frontend_package':
        color = '#8b5cf6';
        break;
      case 'app':
      case 'sub':
      case 'main':
        color = 'var(--color-warning)';
        break;
      case 'component':
      case 'lib':
        color = 'var(--color-primary)';
        break;
      case 'base':
        color = 'var(--color-success)';
        break;
    }
    return {
      backgroundColor: type === 'base' ? 'rgba(16, 185, 129, 0.1)' : 'rgba(255, 255, 255, 0.05)',
      color: color,
      border: `1px solid ${color}`,
      borderColor: 'rgba(255, 255, 255, 0.1)',
    };
  };

  const getTypeDisplayName = (version: string, type: string) => {
    if (version === 'v3') {
      const map: Record<string, string> = { base: '主应用', app: '子应用', lib: '组件' };
      return map[type] || type;
    } else if (version === 'v2') {
      const map: Record<string, string> = { main: '主应用', sub: '子应用', component: '组件' };
      return map[type] || type;
    } else if (version === 'managed') {
      const map: Record<string, string> = {
        frontend_app: '前端应用',
        backend_service: '后端服务',
        frontend_package: '前端共享包',
      };
      return map[type] || type;
    }
    return type;
  };

  const getProvisionBadgeStyle = (status?: string) => {
    if (status === 'degraded') {
      return {
        backgroundColor: 'rgba(239, 68, 68, 0.1)',
        color: 'var(--color-danger)',
        border: '1px solid rgba(239, 68, 68, 0.2)',
      };
    }
    if (status === 'ready') {
      return {
        backgroundColor: 'rgba(16, 185, 129, 0.1)',
        color: 'var(--color-success)',
        border: '1px solid rgba(16, 185, 129, 0.2)',
      };
    }
    return {
      backgroundColor: 'rgba(245, 158, 11, 0.1)',
      color: 'var(--color-warning)',
      border: '1px solid rgba(245, 158, 11, 0.2)',
    };
  };

  const toggleLogs = () => {
    setShowLogs(prev => {
      const next = !prev;
      if (next) setFollowLogs(true);
      return next;
    });
  };

  const terminalIcon = (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M4 17l6-6-6-6" />
      <path d="M12 19h8" />
    </svg>
  );

  const logsIcon = (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M4 4h16v16H4z" />
      <path d="M8 8h8" />
      <path d="M8 12h8" />
      <path d="M8 16h6" />
    </svg>
  );

  return (
    <div
      className="card card-hover flex flex-col"
      style={{
        backgroundColor: project.is_valid ? 'var(--color-surface)' : 'rgba(239, 68, 68, 0.05)',
        borderColor: project.is_valid 
          ? (isRunning ? 'var(--color-success)' : 'var(--color-border)') 
          : 'var(--color-danger)',
        opacity: project.is_valid ? 1 : 0.8,
        transition: 'all 0.2s ease-in-out',
      }}
    >
      {/* Header Section */}
      <div className="flex items-start mb-md" style={{ paddingBottom: '8px', borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
        <div className="flex flex-col gap-xs w-full">
          <div className="flex justify-between items-center gap-sm">
            <h4
              className="m-0 text-lg font-semibold" 
              style={{ 
                color: 'var(--color-text-main)',
                ...(canOpenByPort ? { cursor: 'pointer', transition: 'color 0.2s' } : {})
              }} 
              title={canOpenByPort ? `点击打开 http://localhost:${project.port}` : project.name}
              onClick={canOpenByPort ? () => open(`http://localhost:${project.port}`) : undefined}
              onMouseEnter={(e) => canOpenByPort && (e.currentTarget.style.color = 'var(--color-primary)')}
              onMouseLeave={(e) => canOpenByPort && (e.currentTarget.style.color = 'var(--color-text-main)')}
            >
              {project.name}
            </h4>
            {isRunning && (
              <span 
                className="text-xs flex items-center gap-1"
                style={{ 
                  color: 'var(--color-success)',
                  backgroundColor: 'rgba(16, 185, 129, 0.1)',
                  padding: '2px 8px',
                  borderRadius: '12px',
                  border: '1px solid rgba(16, 185, 129, 0.2)'
                }}
              >
                <span className="animate-spin" style={{ width: 6, height: 6, borderRadius: '50%', border: '2px solid currentColor', borderTopColor: 'transparent', marginRight: 4 }}></span>
                Running
              </span>
            )}
           {project.is_valid && project.runnable && (
            <label 
              className="flex items-center gap-1 text-xs text-secondary cursor-pointer" 
              style={{ userSelect: 'none' }}
            >
              <input
                type="checkbox"
                checked={project.enabled ?? true}
                onChange={handleToggleEnabled}
                style={{ accentColor: 'var(--color-primary)' }}
              />
              <span>批量启动</span>
            </label>
          )}
          </div>
          <div className="project-card__meta mt-xs">
            <div className="project-card__badges">
              <span className="badge" style={getVersionBadgeStyle(project.version)}>
              {project.version.toUpperCase()}
              </span>
              <span className="badge" style={getTypeBadgeStyle(project.type)}>
              {getTypeDisplayName(project.version, project.type)}
              </span>
              {project.provision_status && (
                <span className="badge" style={getProvisionBadgeStyle(project.provision_status)}>
                  {project.provision_status}
                </span>
              )}
            </div>

            {gitStatus ? (
              <div className="project-card__badges project-card__badges--right">
                {gitStatus.branch && (
                  <div ref={branchPickerRef} style={{ position: 'relative' }}>
                    <button
                      type="button"
                      className="badge project-card__badge--truncate"
                      style={{
                        backgroundColor: 'rgba(34, 197, 94, 0.1)',
                        color: 'rgba(34, 197, 94, 0.95)',
                        border: '1px solid rgba(34, 197, 94, 0.25)',
                        cursor: 'pointer',
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '4px',
                      }}
                      title={`分支: ${gitStatus.branch}`}
                      onClick={() => void handleToggleBranchPicker()}
                    >
                      <span>⎇ {gitStatus.branch}</span>
                      <span style={{ opacity: 0.8 }}>{showBranchPicker ? '▴' : '▾'}</span>
                    </button>

                    {showBranchPicker && (
                      <div
                        style={{
                          position: 'absolute',
                          top: 'calc(100% + 8px)',
                          right: 0,
                          width: '320px',
                          maxHeight: '420px',
                          overflow: 'hidden',
                          backgroundColor: 'var(--color-surface)',
                          border: '1px solid var(--color-border)',
                          borderRadius: '12px',
                          boxShadow: 'var(--shadow-xl)',
                          zIndex: 30,
                          display: 'flex',
                          flexDirection: 'column',
                        }}
                      >
                        <div
                          style={{
                            padding: '12px',
                            borderBottom: '1px solid rgba(255,255,255,0.06)',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '8px',
                          }}
                        >
                          <div className="flex items-center justify-between gap-sm">
                            <div>
                              <div className="text-xs text-secondary">切换分支</div>
                              <div className="text-sm font-semibold">{project.name}</div>
                            </div>
                            <button
                              type="button"
                              className="btn btn-ghost btn-sm"
                              onClick={() => void loadBranchPicker()}
                              disabled={branchListLoading || Boolean(branchSwitchingName)}
                            >
                              {branchListLoading ? '刷新中...' : '刷新'}
                            </button>
                          </div>
                          <input
                            type="text"
                            className="input"
                            value={branchSearch}
                            onChange={(event) => setBranchSearch(event.target.value)}
                            placeholder="搜索本地或远端分支"
                            style={{ height: '36px' }}
                          />
                          <div className="text-xs text-muted">
                            打开时会自动 `fetch`。点远端分支会自动创建本地跟踪分支。
                          </div>
                          {branchListError && (
                            <div
                              className="text-xs text-danger"
                              style={{
                                backgroundColor: 'rgba(239, 68, 68, 0.08)',
                                border: '1px solid rgba(239, 68, 68, 0.18)',
                                borderRadius: '8px',
                                padding: '8px',
                              }}
                            >
                              {branchListError}
                            </div>
                          )}
                        </div>

                        <div style={{ overflowY: 'auto', padding: '8px 0' }}>
                          {branchListLoading && (
                            <div className="text-sm text-secondary" style={{ padding: '0 12px 12px' }}>
                              正在刷新分支列表...
                            </div>
                          )}
                          <BranchGroup
                            title="Local"
                            branches={filteredBranches.local}
                            switchingName={branchSwitchingName}
                            onSelect={handleSwitchBranch}
                          />
                          <BranchGroup
                            title="Remote"
                            branches={filteredBranches.remote}
                            switchingName={branchSwitchingName}
                            onSelect={handleSwitchBranch}
                          />

                          {!branchListLoading &&
                            filteredBranches.local.length === 0 &&
                            filteredBranches.remote.length === 0 && (
                              <div className="text-sm text-secondary" style={{ padding: '16px 12px' }}>
                                未找到匹配的分支。
                              </div>
                            )}
                        </div>
                      </div>
                    )}
                  </div>
                )}
                {gitStatus.uncommitted_count > 0 && (
                  <span
                    className="badge"
                    style={{
                      backgroundColor: 'rgba(245, 158, 11, 0.1)',
                      color: 'rgba(245, 158, 11, 0.95)',
                      border: '1px solid rgba(245, 158, 11, 0.25)',
                    }}
                    title="未提交更改"
                  >
                    ✎ {gitStatus.uncommitted_count}
                  </span>
                )}
                {gitStatus.has_remote && (gitStatus.ahead_count > 0 || gitStatus.behind_count > 0) && (
                  <span
                    className="badge"
                    style={{
                      backgroundColor: 'rgba(96, 165, 250, 0.1)',
                      color: 'rgba(96, 165, 250, 0.95)',
                      border: '1px solid rgba(96, 165, 250, 0.25)',
                    }}
                    title="与远端差异"
                  >
                    ↑{gitStatus.ahead_count} ↓{gitStatus.behind_count}
                  </span>
                )}
              </div>
            ) : null}
          </div>

         
        </div>

  
      </div>

      {/* Info Grid */}
      <div 
        className="mb-md" 
        style={{ 
          display: 'grid', 
          gridTemplateColumns: '1fr 1fr', 
          gap: '12px 16px',
          fontSize: '0.875rem'
        }}
      >
        <InfoItem label="平台" value={project.platform} />
        {project.repo_role && <InfoItem label="角色" value={getTypeDisplayName('managed', project.repo_role)} />}
        {project.domain && <InfoItem label="域名" value={project.domain} />}
        {project.framework && <InfoItem label="框架" value={project.framework} />}
        {project.port > 0 && (
          <InfoItem 
            label="端口" 
            value={String(project.port)} 
            valueStyle={{ fontFamily: 'monospace', fontWeight: 'bold', color: 'var(--color-success)' }}
          />
        )}
      </div>
      
      {/* Path & Error */}
       <div className="mb-md">
          <div 
            className="text-xs text-muted"
            style={{ 
              backgroundColor: 'rgba(0,0,0,0.2)', 
              padding: '6px', 
              borderRadius: '4px',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              fontFamily: 'monospace'
            }}
            title={project.path}
          >
            {project.path}
          </div>
          {((!project.is_valid) || project.provision_status === 'degraded') && project.error && (
            <div className="mt-sm p-sm rounded text-danger text-xs" style={{ backgroundColor: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.2)' }}>
              ⚠️ {project.error}
            </div>
          )}
       </div>

	      {/* Actions Footer */}
	      {project.is_valid && (
	        <div className="mt-auto project-card__actions">
	          <div className="project-card__actions-main">
	            <button
	              onClick={canControlProcess ? (isRunning ? handleStop : handleStart) : undefined}
	              disabled={isStarting || !canControlProcess}
	              className={`btn project-card__btn-primary ${isRunning ? 'btn-danger' : 'btn-success'}`}
	              style={{ fontWeight: 600 }}
                title={canControlProcess ? undefined : '受管项目仓库默认不支持直接启动'}
	            >
	              {isStarting ? (
	                 <>
	                  <span className="animate-spin" style={{ width: 14, height: 14, borderRadius: '50%', border: '2px solid currentColor', borderTopColor: 'transparent' }}></span>
                  {isRunning ? '停止中...' : '启动中...'}
                </>
              ) : canControlProcess ? (isRunning ? '停止运行' : '启动项目') : '不支持直接启动'}
            </button>

            {isRunning && canControlProcess && (
              <button
                onClick={handleRestart}
                disabled={isStarting}
                className="btn btn-secondary"
                style={{ padding: '0.5rem 0.75rem', fontWeight: 600, color: 'var(--color-warning)', borderColor: 'var(--color-warning)', backgroundColor: 'rgba(245, 158, 11, 0.12)' }}
                title="重启项目"
              >
                重启
	              </button>
	            )}
	          </div>

	          <div className="project-card__actions-tools">
	            <div className="project-card__actions-icons">
	              {!isManagedProject && (
                <button
	                onClick={() => setShowDebugConfig(!showDebugConfig)}
	                className={`btn project-card__icon-btn ${showDebugConfig ? 'btn-primary' : 'btn-secondary'}`}
	                title="调试依赖配置"
	                aria-label="调试依赖配置"
	              >
	                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.74v-.47a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
	              </button>
                )}
	             
	              {(isRunning || logs.length > 0) && (
	                <button
	                  onClick={toggleLogs}
	                  className={`btn project-card__icon-btn ${showLogs ? 'btn-secondary' : 'btn-ghost'}`}
	                  style={{ borderColor: showLogs ? 'var(--color-border)' : 'transparent', backgroundColor: showLogs ? 'var(--color-surface-hover)' : 'transparent' }}
	                  title="查看日志"
	                  aria-label="查看日志"
	                >
	                  {logsIcon}
	                </button>
	              )}

	              <button
	                onClick={() => setShowTerminal((prev) => !prev)}
	                className={`btn project-card__icon-btn ${showTerminal ? 'btn-primary' : 'btn-secondary'}`}
	                title="终端"
	                aria-label="终端"
	              >
	                {terminalIcon}
	              </button>
	            </div>

	            {gitStatus && !gitDisabledReason && (
	              <div className="project-card__actions-git">
	              <button
	                onClick={handleGitFetch}
	                disabled={isStarting || gitBusy?.fetching || gitBusy?.pulling}
	                className={`btn btn-sm ${gitBusy?.fetching ? 'btn-primary' : 'btn-secondary'}`}
	              >
	                {gitBusy?.fetching ? 'Fetch 中...' : 'Fetch'}
	              </button>
	              <button
                onClick={handleGitPull}
                disabled={
                  isStarting ||
                  gitBusy?.fetching ||
                  gitBusy?.pulling ||
                  !gitStatus.has_remote ||
	                  gitStatus.uncommitted_count > 0
	                }
	                className={`btn btn-sm ${gitBusy?.pulling ? 'btn-primary' : 'btn-secondary'}`}
	                title={gitStatus.uncommitted_count > 0 ? '存在未提交更改，已禁用 Pull' : 'Pull（ff-only）'}
	              >
	                {gitBusy?.pulling ? 'Pull 中...' : 'Pull'}
	              </button>
	              </div>
	            )}
	          </div>

	          {gitStatus && gitDisabledReason && (
	            <div className="text-xs text-muted" title={gitDisabledReason}>
	              Git 功能已禁用：{gitDisabledReason}
	            </div>
	          )}

	          {/* Debug Configuration Panel */}
	          {showDebugConfig && !isManagedProject && (
	             <div style={{ 
	               backgroundColor: 'rgba(0,0,0,0.2)', 
               borderRadius: 'var(--radius-md)', 
               padding: '12px', 
               border: '1px solid var(--color-border)' 
             }}>
              <div className="text-xs font-semibold text-primary mb-sm flex items-center gap-xs">
                <span style={{ width: 4, height: 14, backgroundColor: 'var(--color-primary)', borderRadius: 2 }}></span>
                调试依赖配置
              </div>
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '12px' }}>
                {Object.keys(debugConfig).length === 0 ? (
                  <div className="text-xs text-muted" style={{ textAlign: 'center', padding: '8px', border: '1px dashed var(--color-border)', borderRadius: '4px' }}>
                    暂无已配置的依赖
                  </div>
                ) : (
                  Object.entries(debugConfig).map(([name, url]) => (
                    <div key={name} className="flex items-center justify-between" style={{ backgroundColor: 'rgba(0,0,0,0.2)', padding: '6px 8px', borderRadius: '4px' }}>
                      <div style={{ overflow: 'hidden' }}>
                         <div className="text-xs font-semibold" style={{ color: 'var(--color-text-main)' }}>{name}</div>
                         <div className="text-xs text-muted truncate" style={{ fontFamily: 'monospace', fontSize: '10px' }}>{url}</div>
                      </div>
                      <button
                        onClick={() => handleRemoveDebugDep(name)}
                        className="text-xs text-danger"
                        style={{ padding: '4px', background: 'none', border: 'none', cursor: 'pointer' }}
                        title="移除"
                      >
                        ✕
                      </button>
                    </div>
                  ))
                )}
              </div>

              {availableProjects.length > 0 && (
                <div className="flex gap-xs" style={{ paddingTop: '8px', borderTop: '1px solid rgba(255,255,255,0.05)' }}>
                  <select
                    className="select text-xs"
                    style={{ padding: '4px 8px', height: '28px' }}
                    value={selectedProject}
                    onChange={(e) => setSelectedProject(e.target.value)}
                  >
                    <option value="">选择项目...</option>
                    {availableProjects.map(p => (
                      <option key={p.id} value={p.name}>{p.name}</option>
                    ))}
                  </select>
                  <button
                    onClick={handleAddDebugDep}
                    disabled={!selectedProject}
                    className="btn btn-primary"
                    style={{ padding: '0 10px', height: '28px' }}
                  >
                    +
                  </button>
                </div>
              )}
            </div>
          )}

          {showTerminal && (
            <TerminalPanel
              projectId={project.id}
              projectPath={project.path}
              showQuickCommands={!isManagedProject}
            />
          )}

          {/* Logs Panel */}
          {showLogs && (isRunning || logs.length > 0) && (
            <div style={{ 
              borderRadius: 'var(--radius-md)', 
              overflow: 'hidden', 
              border: '1px solid var(--color-border)', 
              backgroundColor: '#0c0c0c',
              display: 'flex',
              flexDirection: 'column'
            }}>
               <div className="flex justify-between items-center" style={{ padding: '6px 10px', backgroundColor: 'rgba(255,255,255,0.05)', borderBottom: '1px solid var(--color-border)' }}>
                <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', letterSpacing: '0.05em', fontWeight: 600 }}>CONSOLE OUTPUT</span>
                <div className="flex items-center gap-sm">
                  <button
                    onClick={() => setFollowLogs(!followLogs)}
                    style={{ fontSize: '10px', color: 'var(--color-text-secondary)', background: 'none', border: 'none', cursor: 'pointer' }}
                  >
                    {followLogs ? '取消跟随' : '跟随输出'}
                  </button>
                  <button 
                    onClick={() => setShowLogs(false)} 
                    style={{ fontSize: '10px', color: 'var(--color-text-secondary)', background: 'none', border: 'none', cursor: 'pointer' }}
                  >
                    收起
                  </button>
                  <button 
                    onClick={() => setLogs([])} 
                    style={{ fontSize: '10px', color: 'var(--color-text-secondary)', background: 'none', border: 'none', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '4px' }}
                  >
                    Clear
                  </button>
                </div>
              </div>
              <div
                ref={logContainerRef}
                onScroll={handleLogsScroll}
                style={{
                  padding: '10px',
                  fontFamily: 'monospace',
                  fontSize: '11px',
                  lineHeight: '1.5',
                  height: '200px',
                  overflowY: 'auto',
                  color: '#e5e7eb'
                }}
              >
                {logs.length === 0 ? (
                  <div style={{ color: '#6b7280', fontStyle: 'italic' }}>等待输出...</div>
                ) : (
                  logs.map((log, idx) => (
                    <div
                      key={idx}
                      style={{
                        color: log.stream === 'stderr' ? '#f87171' : '#4ade80',
                        wordBreak: 'break-all',
                        marginBottom: '2px'
                      }}
                    >
                      <span style={{ color: '#4b5563', marginRight: '8px', userSelect: 'none' }}>[{new Date().toLocaleTimeString()}]</span>
                      {log.message}
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function BranchGroup({
  title,
  branches,
  switchingName,
  onSelect,
}: {
  title: string;
  branches: GitBranch[];
  switchingName: string | null;
  onSelect: (branch: GitBranch) => Promise<void>;
}) {
  if (branches.length === 0) {
    return null;
  }

  return (
    <div style={{ padding: '0 8px 8px' }}>
      <div
        className="text-xs text-secondary"
        style={{
          padding: '8px 4px 6px',
          textTransform: 'uppercase',
          letterSpacing: '0.06em',
        }}
      >
        {title}
      </div>
      <div className="grid gap-xs">
        {branches.map((branch) => {
          const isSwitching = switchingName === branch.name;
          const isCurrentLocal = !branch.is_remote && branch.is_current;
          return (
            <button
              key={`${title}-${branch.name}`}
              type="button"
              onClick={() => void onSelect(branch)}
              disabled={isSwitching || isCurrentLocal}
              className="btn btn-ghost"
              style={{
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '0.55rem 0.75rem',
                backgroundColor: isCurrentLocal ? 'rgba(34, 197, 94, 0.08)' : 'transparent',
                border: '1px solid rgba(255,255,255,0.05)',
                color: 'var(--color-text-main)',
              }}
              title={branch.is_remote ? '切换到远端分支并创建本地跟踪分支' : `切换到 ${branch.name}`}
            >
              <span
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  minWidth: 0,
                  overflow: 'hidden',
                }}
              >
                <span style={{ color: branch.is_remote ? 'var(--color-primary)' : 'var(--color-success)' }}>
                  {branch.is_remote ? '☁' : branch.is_current ? '✓' : '⎇'}
                </span>
                <span
                  style={{
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    textAlign: 'left',
                  }}
                >
                  {branch.name}
                </span>
              </span>
              <span className="text-xs text-secondary" style={{ flexShrink: 0, marginLeft: '8px' }}>
                {isSwitching ? '切换中...' : branch.is_remote ? 'track' : branch.is_current ? '当前' : branch.upstream ?? ''}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// Helper Component for Grid Items
function InfoItem({ label, value, valueStyle = {} }: { label: string, value: string, valueStyle?: React.CSSProperties }) {
  return (
    <div className="flex flex-col" style={{ gap: '2px' }}>
      <span className="text-xs text-secondary">{label}</span>
      <span className="text-sm" style={{ color: 'var(--color-text-main)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', ...valueStyle }}>{value}</span>
    </div>
  );
}

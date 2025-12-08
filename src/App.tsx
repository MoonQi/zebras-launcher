import { useState, useEffect } from 'react';
import { useWorkspace } from './hooks/useWorkspace';
import { WorkspaceManager } from './components/workspace/WorkspaceManager';
import { ProjectGrid } from './components/workspace/ProjectGrid';
import { getWorkspaceList, loadWorkspace, deleteWorkspace, startAllProjects, stopAllProjects } from './services/tauri';
import type { PortChange, WorkspaceRef, ProcessInfo } from './types';

function App() {
  const {
    workspace,
    loading,
    error,
    selectAndCreateWorkspace,
    rescanProjects,
    resolveConflicts,
    addFolder,
    removeFolder,
    setError,
    setWorkspace,
  } = useWorkspace();

  const [portChanges, setPortChanges] = useState<PortChange[]>([]);
  const [showPortChanges, setShowPortChanges] = useState(false);
  const [workspaceList, setWorkspaceList] = useState<WorkspaceRef[]>([]);
  const [loadingList, setLoadingList] = useState(false);
  const [runningProcesses, setRunningProcesses] = useState<Map<string, ProcessInfo>>(new Map());

  // 加载工作区列表
  useEffect(() => {
    loadWorkspaceList();
  }, []);

  const loadWorkspaceList = async () => {
    try {
      const list = await getWorkspaceList();
      setWorkspaceList(list);
    } catch (err) {
      console.error('加载工作区列表失败:', err);
    }
  };

  const handleWorkspaceChange = async (configPath: string) => {
    // 如果选择"-- 选择工作区 --"（空值），清空当前工作区
    if (!configPath) {
      setWorkspace(null);
      return;
    }

    try {
      setLoadingList(true);
      const ws = await loadWorkspace(configPath);
      setWorkspace(ws);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingList(false);
    }
  };

  const handleCreateWorkspace = async (name: string) => {
    await selectAndCreateWorkspace(name);
    await loadWorkspaceList(); // 刷新工作区列表
  };

  const handleDeleteWorkspace = async () => {
    if (!workspace) return;

    try {
      setError(null);
      // 调用删除命令
      await deleteWorkspace(workspace.id, workspace.root_path);

      // 清空当前工作区
      setWorkspace(null);

      // 刷新工作区列表
      await loadWorkspaceList();

      alert('工作区已成功删除！');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRescan = async () => {
    await rescanProjects();
  };

  const handleResolveConflicts = async () => {
    const changes = await resolveConflicts();
    if (changes.length > 0) {
      setPortChanges(changes);
      setShowPortChanges(true);
    } else {
      alert('未发现端口冲突！');
    }
  };

  const handleStartAll = async () => {
    if (!workspace) return;

    try {
      setError(null);
      const processes = await startAllProjects(workspace);

      // 更新运行进程映射
      const newProcesses = new Map(runningProcesses);
      processes.forEach(proc => {
        newProcesses.set(proc.project_id, proc);
      });
      setRunningProcesses(newProcesses);

      alert(`成功启动 ${processes.length} 个项目！`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleProcessStart = (projectId: string, processInfo: ProcessInfo) => {
    setRunningProcesses(prev => new Map(prev).set(projectId, processInfo));
  };

  const handleProcessStop = (projectId: string) => {
    setRunningProcesses(prev => {
      const newMap = new Map(prev);
      newMap.delete(projectId);
      return newMap;
    });
  };

  const handleStopAll = async () => {
    try {
      setError(null);
      await stopAllProjects();
      setRunningProcesses(new Map()); // 清空运行状态
      alert('已停止所有运行中的项目！');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // 处理调试配置变更，重新扫描项目
  const handleDebugConfigChange = async () => {
    if (workspace) {
      try {
        await rescanProjects();
      } catch (err) {
        console.error('重新扫描项目失败:', err);
      }
    }
  };

  // 处理工作区更新（如项目启用状态变更）
  const handleWorkspaceUpdate = (updatedWorkspace: typeof workspace) => {
    setWorkspace(updatedWorkspace);
  };

  // 检查是否所有有效且启用的项目都在运行
  const allProjectsRunning = workspace ?
    workspace.projects
      .filter(p => p.is_valid && (p.enabled ?? true)) // 只检查启用的项目
      .every(p => runningProcesses.has(p.id)) &&
    workspace.projects.filter(p => p.is_valid && (p.enabled ?? true)).length > 0
    : false;

  const selectedWorkspaceConfigPath = workspace
    ? workspaceList.find((ws) => ws.id === workspace.id)?.config_path ?? ''
    : '';

  return (
    <div>
      {/* 头部 */}
      <header className="flex justify-between items-center mb-lg">
        <div>
          <h1 className="text-primary">Zebras Launcher</h1>
          <p>Zebras 前端工程启动器 - 轻松管理 Zebras2.0/3.0 项目</p>
        </div>

        {/* 工作区选择下拉框 */}
        {workspaceList.length > 0 && (
          <div style={{ minWidth: '250px' }}>
            <label className="block mb-sm text-secondary text-sm">
              选择工作区:
            </label>
            <select
              className="select"
              value={selectedWorkspaceConfigPath}
              onChange={(e) => handleWorkspaceChange(e.target.value)}
              disabled={loadingList || loading}
            >
              <option value="">-- 选择工作区 --</option>
              {workspaceList.map((ws) => (
                <option key={ws.id} value={ws.config_path}>
                  {ws.name}
                </option>
              ))}
            </select>
          </div>
        )}
      </header>

      {/* 错误提示 */}
      {error && (
        <div className="card mb-lg flex justify-between items-center" 
             style={{ backgroundColor: 'rgba(239, 68, 68, 0.1)', borderColor: 'rgba(239, 68, 68, 0.2)' }}>
          <div>
            <strong className="text-danger">错误：</strong>
            <span className="ml-sm text-danger">{error}</span>
          </div>
          <button
            onClick={() => setError(null)}
            className="btn-ghost text-danger"
            style={{ fontSize: '1.25rem', padding: '0.25rem 0.5rem' }}
          >
            ×
          </button>
        </div>
      )}

      {/* 端口变更提示 */}
      {showPortChanges && portChanges.length > 0 && (
        <div className="card mb-lg"
             style={{ backgroundColor: 'rgba(16, 185, 129, 0.1)', borderColor: 'rgba(16, 185, 129, 0.2)' }}>
          <div className="flex justify-between items-start mb-sm">
            <div>
              <strong className="text-success">端口冲突已解决！</strong>
              <p className="mt-sm text-success text-sm">
                以下项目的端口已自动调整并更新到本地配置文件：
              </p>
            </div>
            <button
              onClick={() => setShowPortChanges(false)}
              className="btn-ghost text-success"
              style={{ fontSize: '1.25rem', padding: '0.25rem 0.5rem' }}
            >
              ×
            </button>
          </div>
          <ul className="pl-lg text-success">
            {portChanges.map((change, idx) => (
              <li key={idx} className="mb-xs">
                <strong>{change.project_name}</strong>: {change.old_port} → {change.new_port}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* 工作区管理 */}
      <WorkspaceManager
        workspace={workspace}
        onCreateWorkspace={handleCreateWorkspace}
        onDeleteWorkspace={handleDeleteWorkspace}
        onRescan={handleRescan}
        onResolveConflicts={handleResolveConflicts}
        onAddFolder={addFolder}
        onRemoveFolder={removeFolder}
        onStartAll={handleStartAll}
        onStopAll={handleStopAll}
        loading={loading}
        allProjectsRunning={allProjectsRunning}
      />

      {/* 项目网格 */}
      {workspace && (
        <ProjectGrid
          projects={workspace.projects}
          runningProcesses={runningProcesses}
          onProcessStart={handleProcessStart}
          onProcessStop={handleProcessStop}
          onDebugConfigChange={handleDebugConfigChange}
          workspace={workspace}
          onWorkspaceUpdate={handleWorkspaceUpdate}
        />
      )}

      {/* 加载指示器 */}
      {loading && (
        <div
          className="card flex items-center gap-md"
          style={{
            position: 'fixed',
            bottom: '20px',
            right: '20px',
            boxShadow: 'var(--shadow-xl)',
            zIndex: 100
          }}
        >
          <div
            className="animate-spin"
            style={{
              width: '20px',
              height: '20px',
              border: '3px solid var(--color-primary)',
              borderTopColor: 'transparent',
              borderRadius: '50%',
            }}
          />
          <span className="text-primary">处理中...</span>
        </div>
      )}
    </div>
  );
}

export default App;

import { ProjectCard } from '../project/ProjectCard';
import type { GitPullResult, GitStatus, ProjectInfo, ProcessInfo, Workspace } from '../../types';

interface ProjectGridProps {
  projects: ProjectInfo[];
  runningProcesses: Map<string, ProcessInfo>;
  onProcessStart: (projectId: string, processInfo: ProcessInfo) => void;
  onProcessStop: (projectId: string) => void;
  onDebugConfigChange: () => void; // 调试配置变更回调
  workspace: Workspace; // 添加 workspace 参数
  onWorkspaceUpdate: (workspace: Workspace) => void; // 工作区更新回调
  gitStatuses: Map<string, GitStatus | null>;
  gitBusyByProjectId: Map<string, { fetching: boolean; pulling: boolean }>;
  gitDisabledReason: string | null;
  onGitFetch: (project: ProjectInfo) => Promise<void>;
  onGitPull: (project: ProjectInfo) => Promise<GitPullResult | null>;
}

export function ProjectGrid({
  projects,
  runningProcesses,
  onProcessStart,
  onProcessStop,
  workspace,
  onWorkspaceUpdate,
  gitStatuses,
  gitBusyByProjectId,
  gitDisabledReason,
  onGitFetch,
  onGitPull,
}: ProjectGridProps) {
  if (projects.length === 0) {
    return (
      <div className="text-center py-xl text-secondary">
        <p className="text-lg mb-sm">未发现任何 Zebras 项目</p>
        <p className="text-sm">
          请确保选择的目录包含 zebra.json 或 zebras.config.ts 配置文件
        </p>
      </div>
    );
  }

  // 按类型分组统计
  const stats = {
    total: projects.length,
    valid: projects.filter((p) => p.is_valid).length,
    invalid: projects.filter((p) => !p.is_valid).length,
    v2: projects.filter((p) => p.version === 'v2').length,
    v3: projects.filter((p) => p.version === 'v3').length,
    // 主应用：base(v3) + main(v2)
    mainApps: projects.filter((p) => p.type === 'base' || p.type === 'main').length,
    // 子应用：app(v3) + sub(v2)
    subApps: projects.filter((p) => p.type === 'app' || p.type === 'sub').length,
    // 组件：lib(v3) + component(v2)
    components: projects.filter((p) => p.type === 'lib' || p.type === 'component').length,
  };

  return (
    <div>
      {/* 统计信息 */}
      <div className="card mb-lg flex items-center p-md bg-surface">
        <div className="flex-1 flex justify-center gap-lg">
          <StatItem label="总计" value={stats.total} color="var(--color-primary)" />
          <StatItem label="有效" value={stats.valid} color="var(--color-success)" />
          {stats.invalid > 0 && <StatItem label="错误" value={stats.invalid} color="var(--color-danger)" />}
        </div>
        
        <div className="flex-1 flex justify-center gap-lg border-l border-border" style={{ borderLeft: '1px solid var(--color-border)' }}>
          <StatItem label="V2" value={stats.v2} color="#60a5fa" />
          <StatItem label="V3" value={stats.v3} color="#4ade80" />
        </div>
        
        <div className="flex-1 flex justify-center gap-lg border-l border-border" style={{ borderLeft: '1px solid var(--color-border)' }}>
          <StatItem label="主应用" value={stats.mainApps} color="#10b981" />
          <StatItem label="子应用" value={stats.subApps} color="#f59e0b" />
          <StatItem label="组件" value={stats.components} color="#8b5cf6" />
        </div>
      </div>

      {/* 项目网格 */}
      <div
        className="grid"
        style={{
          gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))',
          gap: '20px',
        }}
      >
        {projects.map((project) => (
          <ProjectCard
            key={project.id}
            project={project}
            processInfo={runningProcesses.get(project.id)}
            onProcessStart={onProcessStart}
            onProcessStop={onProcessStop}
            allProjects={projects}
            workspace={workspace}
            onWorkspaceUpdate={onWorkspaceUpdate}
            gitStatus={gitStatuses.get(project.id)}
            gitBusy={gitBusyByProjectId.get(project.id)}
            gitDisabledReason={gitDisabledReason}
            onGitFetch={onGitFetch}
            onGitPull={onGitPull}
          />
        ))}
      </div>
    </div>
  );
}
interface StatItemProps {
  label: string;
  value: number;
  color: string;
}

function StatItem({ label, value, color }: StatItemProps) {
  return (
    <div className="text-center">
      <div className="font-bold" style={{ fontSize: '1.5rem', color }}>{value}</div>
      <div className="text-xs text-secondary mt-xs">{label}</div>
    </div>
  );
}


import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  ReactFlowProvider,
  applyEdgeChanges,
  applyNodeChanges,
  MarkerType,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type OnConnect,
} from 'reactflow';
import type { ProcessInfo, ProjectInfo } from '../../types';
import { updateDebugConfig } from '../../services/tauri';

type DebugMap = Record<string, string>;

type DebugEdgeData = {
  kind: 'debug';
  sourceProjectId: string;
  depName: string;
};

type ProjectNodeData = {
  kind: 'project';
  projectId: string;
};

type ExternalNodeData = {
  kind: 'external';
  name: string;
};

type NodeData = (ProjectNodeData | ExternalNodeData) & { label?: ReactNode };

interface DependencyGraphModalProps {
  projects: ProjectInfo[];
  runningProcesses: Map<string, ProcessInfo>;
  onClose: () => void;
  onDebugConfigSaved: () => Promise<void>;
  onRestartRunningProjects: (projectIds: string[]) => Promise<void>;
}

function getDefaultDebugUrl(project: ProjectInfo) {
  return `http://localhost:${project.port}`;
}

function getProjectTypeMeta(project: ProjectInfo) {
  const isV3 = project.version === 'v3';
  const isV2 = project.version === 'v2';

  const isMain = (isV3 && project.type === 'base') || (isV2 && project.type === 'main');
  const isSub = (isV3 && project.type === 'app') || (isV2 && project.type === 'sub');
  const isComponent = (isV3 && project.type === 'lib') || (isV2 && project.type === 'component');

  if (isMain) {
    return {
      key: 'main' as const,
      label: '主应用',
      color: 'var(--color-success)',
      bg: 'rgba(16, 185, 129, 0.12)',
      border: 'rgba(16, 185, 129, 0.35)',
    };
  }
  if (isSub) {
    return {
      key: 'sub' as const,
      label: '子应用',
      color: 'var(--color-warning)',
      bg: 'rgba(245, 158, 11, 0.12)',
      border: 'rgba(245, 158, 11, 0.35)',
    };
  }
  if (isComponent) {
    return {
      key: 'component' as const,
      label: '组件',
      color: '#a78bfa',
      bg: 'rgba(167, 139, 250, 0.12)',
      border: 'rgba(167, 139, 250, 0.35)',
    };
  }
  return {
    key: 'other' as const,
    label: project.type,
    color: 'var(--color-text-secondary)',
    bg: 'rgba(255, 255, 255, 0.06)',
    border: 'rgba(255, 255, 255, 0.12)',
  };
}

function buildInitialDraft(projects: ProjectInfo[]): Record<string, DebugMap> {
  const next: Record<string, DebugMap> = {};
  projects.forEach((project) => {
    next[project.id] = { ...(project.debug ?? {}) };
  });
  return next;
}

function buildRequiredExternalNodeIds(draftByProjectId: Record<string, DebugMap>, projectIdByName: Map<string, string>) {
  const ids = new Set<string>();
  Object.values(draftByProjectId).forEach((debugMap) => {
    Object.keys(debugMap).forEach((depName) => {
      if (!projectIdByName.has(depName)) {
        ids.add(`external:${depName}`);
      }
    });
  });
  return ids;
}

function buildEdges(
  draftByProjectId: Record<string, DebugMap>,
  projectIdByName: Map<string, string>
): Edge<DebugEdgeData>[] {
  const edges: Edge<DebugEdgeData>[] = [];
  Object.entries(draftByProjectId).forEach(([sourceProjectId, debugMap]) => {
    Object.keys(debugMap).forEach((depName) => {
      const targetId = projectIdByName.get(depName) ?? `external:${depName}`;
      edges.push({
        id: `debug:${sourceProjectId}:${depName}`,
        source: sourceProjectId,
        target: targetId,
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, color: 'rgba(59, 130, 246, 0.9)' },
        data: { kind: 'debug', sourceProjectId, depName },
        style: { stroke: 'rgba(59, 130, 246, 0.9)' },
      });
    });
  });
  return edges;
}

function buildInitialNodes(projects: ProjectInfo[], externalNodeIds: Set<string>): Node<NodeData>[] {
  const columns = Math.max(2, Math.ceil(Math.sqrt(projects.length)));
  const nodes: Node<NodeData>[] = projects.map((project, idx) => {
    const x = (idx % columns) * 260;
    const y = Math.floor(idx / columns) * 140;
    const isValid = project.is_valid;
    const typeMeta = getProjectTypeMeta(project);
    return {
      id: project.id,
      position: { x, y },
      data: { kind: 'project', projectId: project.id },
      style: {
        padding: 10,
        borderRadius: 10,
        border: `1px solid ${isValid ? 'var(--color-border)' : 'rgba(239, 68, 68, 0.55)'}`,
        borderLeft: `4px solid ${isValid ? typeMeta.border : 'rgba(239, 68, 68, 0.65)'}`,
        background: isValid
          ? `linear-gradient(90deg, ${typeMeta.bg} 0%, var(--color-surface) 56%)`
          : 'rgba(239, 68, 68, 0.08)',
        color: 'var(--color-text-main)',
        width: 220,
      },
    };
  });

  const external = Array.from(externalNodeIds).map((id, idx) => {
    const name = id.replace(/^external:/, '');
    return {
      id,
      position: { x: columns * 260 + 40, y: idx * 90 },
      data: { kind: 'external', name },
      selectable: true,
      style: {
        padding: 10,
        borderRadius: 10,
        border: '1px dashed rgba(148, 163, 184, 0.6)',
        background: 'rgba(15, 23, 42, 0.25)',
        color: 'var(--color-text-secondary)',
        width: 220,
      },
    } satisfies Node<NodeData>;
  });

  return [...nodes, ...external];
}

export function DependencyGraphModal({
  projects,
  runningProcesses,
  onClose,
  onDebugConfigSaved,
  onRestartRunningProjects,
}: DependencyGraphModalProps) {
  const projectsById = useMemo(() => {
    return new Map(projects.map((p) => [p.id, p]));
  }, [projects]);

  const projectIdByName = useMemo(() => {
    const map = new Map<string, string>();
    projects.forEach((p) => map.set(p.name, p.id));
    return map;
  }, [projects]);

  const [draftByProjectId, setDraftByProjectId] = useState<Record<string, DebugMap>>(() => buildInitialDraft(projects));
  const [dirtyProjectIds, setDirtyProjectIds] = useState<Set<string>>(new Set());
  const [touchedProjectIds, setTouchedProjectIds] = useState<Set<string>>(new Set());
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const requiredExternalNodeIds = useMemo(() => {
    return buildRequiredExternalNodeIds(draftByProjectId, projectIdByName);
  }, [draftByProjectId, projectIdByName]);

  const [nodes, setNodes] = useState<Node<NodeData>[]>(() =>
    buildInitialNodes(projects, buildRequiredExternalNodeIds(buildInitialDraft(projects), projectIdByName))
  );
  const [edges, setEdges] = useState<Edge<DebugEdgeData>[]>(() => buildEdges(buildInitialDraft(projects), projectIdByName));

  useEffect(() => {
    setDraftByProjectId(buildInitialDraft(projects));
    setDirtyProjectIds(new Set());
    setTouchedProjectIds(new Set());

    const initialDraft = buildInitialDraft(projects);
    const externalIds = buildRequiredExternalNodeIds(initialDraft, projectIdByName);
    setNodes(buildInitialNodes(projects, externalIds));
    setEdges(buildEdges(initialDraft, projectIdByName));
    setSelectedNodeId(null);
    setSelectedEdgeId(null);
  }, [projects, projectIdByName]);

  useEffect(() => {
    setNodes((prev) => {
      const existing = new Set(prev.map((n) => n.id));
      const next = [...prev];

      requiredExternalNodeIds.forEach((id) => {
        if (existing.has(id)) return;
        const name = id.replace(/^external:/, '');
        next.push({
          id,
          position: { x: 900, y: 40 + next.length * 16 },
          data: { kind: 'external', name },
          selectable: true,
          style: {
            padding: 10,
            borderRadius: 10,
            border: '1px dashed rgba(148, 163, 184, 0.6)',
            background: 'rgba(15, 23, 42, 0.25)',
            color: 'var(--color-text-secondary)',
            width: 220,
          },
        });
      });

      const required = new Set(requiredExternalNodeIds);
      return next.filter((n) => {
        if (String(n.id).startsWith('external:')) return required.has(String(n.id));
        return true;
      });
    });

    setEdges(buildEdges(draftByProjectId, projectIdByName));
  }, [draftByProjectId, projectIdByName, requiredExternalNodeIds]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
  }, []);

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    const removed = changes.filter((c) => c.type === 'remove').map((c) => c.id);
    if (removed.length === 0) {
      setEdges((eds) => applyEdgeChanges(changes, eds));
      return;
    }

    const removedDebugEdges = removed
      .map((edgeId) => edges.find((e) => e.id === edgeId))
      .filter((edge): edge is Edge<DebugEdgeData> => Boolean(edge?.data && edge.data.kind === 'debug'));

    if (removedDebugEdges.length === 0) return;

    setDraftByProjectId((prev) => {
      const next = { ...prev };
      removedDebugEdges.forEach((edge) => {
        const { sourceProjectId, depName } = edge.data!;
        const existing = next[sourceProjectId] ?? {};
        if (!(depName in existing)) return;
        const updated: DebugMap = { ...existing };
        delete updated[depName];
        next[sourceProjectId] = updated;
      });
      return next;
    });

    setDirtyProjectIds((prev) => {
      const next = new Set(prev);
      removedDebugEdges.forEach((edge) => next.add(edge.data!.sourceProjectId));
      return next;
    });

    setTouchedProjectIds((prev) => {
      const next = new Set(prev);
      removedDebugEdges.forEach((edge) => {
        next.add(edge.data!.sourceProjectId);
        const targetId = projectIdByName.get(edge.data!.depName);
        if (targetId) next.add(targetId);
      });
      return next;
    });
  }, [edges, projectIdByName]);

  const onConnect: OnConnect = useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target) return;
      const sourceId = connection.source;
      const targetId = connection.target;
      if (sourceId === targetId) return;
      if (String(sourceId).startsWith('external:')) return;
      if (String(targetId).startsWith('external:')) return;

      const targetProject = projectsById.get(targetId);
      if (!targetProject) return;

      setDraftByProjectId((prev) => {
        const next = { ...prev };
        const existing = next[sourceId] ?? {};
        if (targetProject.name in existing) return prev;

        next[sourceId] = { ...existing, [targetProject.name]: getDefaultDebugUrl(targetProject) };

        setDirtyProjectIds((dirtyPrev) => new Set(dirtyPrev).add(sourceId));
        setTouchedProjectIds((touchedPrev) => {
          const nextTouched = new Set(touchedPrev);
          nextTouched.add(sourceId);
          nextTouched.add(targetId);
          return nextTouched;
        });

        return next;
      });
    },
    [projectsById]
  );

  const selectedEdge = useMemo(() => {
    if (!selectedEdgeId) return null;
    return edges.find((e) => e.id === selectedEdgeId) ?? null;
  }, [edges, selectedEdgeId]);

  const selectedProject = useMemo(() => {
    if (!selectedNodeId) return null;
    const node = nodes.find((n) => n.id === selectedNodeId);
    if (!node || node.data.kind !== 'project') return null;
    return projectsById.get(node.data.projectId) ?? null;
  }, [nodes, projectsById, selectedNodeId]);

  const selectedEdgeUrl = useMemo(() => {
    if (!selectedEdge?.data) return '';
    const { sourceProjectId, depName } = selectedEdge.data;
    return draftByProjectId[sourceProjectId]?.[depName] ?? '';
  }, [draftByProjectId, selectedEdge]);

  const setSelectedEdgeUrl = useCallback((nextUrl: string) => {
    const data = selectedEdge?.data;
    if (!data) return;
    const { sourceProjectId, depName } = data;
    setDraftByProjectId((prev) => {
      const source = prev[sourceProjectId] ?? {};
      if (source[depName] === nextUrl) return prev;
      return {
        ...prev,
        [sourceProjectId]: { ...source, [depName]: nextUrl },
      };
    });
    setDirtyProjectIds((prev) => new Set(prev).add(sourceProjectId));
    setTouchedProjectIds((prev) => new Set(prev).add(sourceProjectId));
  }, [selectedEdge]);

  const dirtyCount = dirtyProjectIds.size;

  const involvedRunningProjectIds = useMemo(() => {
    const ids = Array.from(touchedProjectIds);
    return ids.filter((id) => runningProcesses.has(id));
  }, [runningProcesses, touchedProjectIds]);

  const persistDraft = useCallback(async () => {
    if (saving) return;
    if (dirtyProjectIds.size === 0) return;

    setSaving(true);
    try {
      const ids = Array.from(dirtyProjectIds);
      for (const projectId of ids) {
        const project = projectsById.get(projectId);
        if (!project) continue;
        const debugMap = draftByProjectId[projectId] ?? {};
        await updateDebugConfig(project.path, project.version, debugMap);
      }
      setDirtyProjectIds(new Set());
    } finally {
      setSaving(false);
    }
  }, [dirtyProjectIds, draftByProjectId, projectsById, saving]);

  const saveDraft = useCallback(async () => {
    await persistDraft();
    await onDebugConfigSaved();
  }, [onDebugConfigSaved, persistDraft]);

  const saveAndRestart = useCallback(async () => {
    const runningIds = involvedRunningProjectIds;
    if (dirtyProjectIds.size === 0) return;
    if (runningIds.length === 0) {
      await saveDraft();
      return;
    }
    if (!confirm(`将保存调试依赖配置，并重启 ${runningIds.length} 个正在运行的项目，是否继续？`)) return;
    await persistDraft();
    await onRestartRunningProjects(runningIds);
    await onDebugConfigSaved();
  }, [dirtyProjectIds.size, involvedRunningProjectIds, onDebugConfigSaved, onRestartRunningProjects, persistDraft, saveDraft]);

  const renderNodeLabel = useCallback(
    (node: Node<NodeData>) => {
      if (node.data.kind === 'external') {
        return (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <div style={{ fontWeight: 600, fontSize: 12 }}>{node.data.name}</div>
            <div className="text-xs text-muted">外部依赖</div>
          </div>
        );
      }

      const project = projectsById.get(node.data.projectId);
      if (!project) return null;
      const typeMeta = getProjectTypeMeta(project);
      const isRunning = runningProcesses.has(project.id);
      return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
            <div style={{ fontWeight: 700, fontSize: 13, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {project.name}
            </div>
            {isRunning ? (
              <span className="badge" style={{ backgroundColor: 'rgba(16, 185, 129, 0.1)', color: 'var(--color-success)', border: '1px solid rgba(16, 185, 129, 0.25)' }}>
                Running
              </span>
            ) : null}
          </div>
          <div className="text-xs text-muted" style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
            <span
              className="badge"
              style={{
                backgroundColor: typeMeta.bg,
                color: typeMeta.color,
                border: `1px solid ${typeMeta.border}`,
              }}
              title={project.type}
            >
              {typeMeta.label}
            </span>
            <span className="badge" style={{ backgroundColor: 'rgba(255,255,255,0.06)', color: 'var(--color-text-secondary)', border: '1px solid rgba(255,255,255,0.1)' }}>
              {project.version.toUpperCase()}
            </span>
            <span style={{ fontFamily: 'monospace', color: 'var(--color-text-secondary)' }}>{project.port}</span>
          </div>
        </div>
      );
    },
    [projectsById, runningProcesses]
  );

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="dependency-graph__overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="card dependency-graph__panel">
        <div className="dependency-graph__header">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <h2 className="m-0" style={{ fontSize: '1.1rem' }}>调试依赖关系图</h2>
            <div className="text-xs text-muted">
              拖拽连线添加依赖，选中连线可编辑 URL，删除连线可移除依赖。
            </div>
          </div>

          <div className="dependency-graph__header-actions">
            <button onClick={onClose} className="btn btn-secondary">关闭</button>
            <button
              onClick={saveDraft}
              disabled={saving || dirtyCount === 0}
              className={`btn ${dirtyCount > 0 ? 'btn-primary' : 'btn-secondary'}`}
              title={dirtyCount === 0 ? '暂无改动' : `保存 ${dirtyCount} 个项目的调试依赖`}
            >
              {saving ? '保存中...' : '保存'}
            </button>
            <button
              onClick={saveAndRestart}
              disabled={saving || dirtyCount === 0}
              className="btn btn-warning"
              title="保存并重启涉及的运行中项目"
            >
              保存并重启
            </button>
          </div>
        </div>

        <div className="dependency-graph__content">
          <div className="dependency-graph__graph">
            <ReactFlowProvider>
              <ReactFlow
                nodes={nodes.map((n) => ({ ...n, data: { ...n.data, label: renderNodeLabel(n) } }))}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onSelectionChange={(sel) => {
                  const node = sel.nodes?.[0]?.id ?? null;
                  const edge = sel.edges?.[0]?.id ?? null;
                  setSelectedNodeId(node);
                  setSelectedEdgeId(edge);
                }}
                fitView
                fitViewOptions={{ padding: 0.18 }}
                defaultEdgeOptions={{ animated: false }}
                proOptions={{ hideAttribution: true }}
              >
                <Background gap={18} size={1} color="rgba(148, 163, 184, 0.18)" />
                <Controls />
                <MiniMap
                  nodeStrokeColor={(n) => {
                    if (String(n.id).startsWith('external:')) return 'rgba(148,163,184,0.6)';
                    const project = projectsById.get(String(n.id));
                    if (!project) return 'rgba(59,130,246,0.6)';
                    return getProjectTypeMeta(project).border;
                  }}
                  nodeColor={(n) => {
                    if (String(n.id).startsWith('external:')) return 'rgba(15,23,42,0.5)';
                    const project = projectsById.get(String(n.id));
                    if (!project) return 'rgba(30,41,59,0.9)';
                    return getProjectTypeMeta(project).bg;
                  }}
                  maskColor="rgba(0,0,0,0.35)"
                />
              </ReactFlow>
            </ReactFlowProvider>
          </div>

          <div className="dependency-graph__side">
            <div className="dependency-graph__side-card">
              <div className="text-xs font-semibold text-secondary" style={{ textTransform: 'uppercase', letterSpacing: '0.06em' }}>
                选择详情
              </div>

              {selectedEdge?.data ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                  <div className="text-sm" style={{ fontWeight: 600 }}>
                    依赖：{projectsById.get(selectedEdge.data.sourceProjectId)?.name ?? selectedEdge.data.sourceProjectId}
                    {' → '}
                    {selectedEdge.data.depName}
                  </div>
                  <div>
                    <label className="block mb-sm text-secondary text-sm">URL</label>
                    <input
                      className="input"
                      value={selectedEdgeUrl}
                      onChange={(e) => setSelectedEdgeUrl(e.target.value)}
                      placeholder="http://localhost:xxxx"
                    />
                    <div className="text-xs text-muted mt-xs">
                      该 URL 会写入源项目的调试依赖配置文件。
                    </div>
                  </div>
                </div>
              ) : selectedProject ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                  <div className="text-sm" style={{ fontWeight: 700 }}>{selectedProject.name}</div>
                  <div className="text-xs text-muted" style={{ fontFamily: 'monospace' }}>{selectedProject.path}</div>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                    <div className="text-xs text-secondary" style={{ fontWeight: 600 }}>当前依赖</div>
                    {Object.keys(draftByProjectId[selectedProject.id] ?? {}).length === 0 ? (
                      <div className="text-xs text-muted">暂无</div>
                    ) : (
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                        {Object.entries(draftByProjectId[selectedProject.id] ?? {}).map(([depName, url]) => (
                          <div key={depName} className="dependency-graph__dep-row">
                            <div style={{ overflow: 'hidden' }}>
                              <div className="text-xs" style={{ fontWeight: 600, color: 'var(--color-text-main)' }}>{depName}</div>
                              <div className="text-xs text-muted" style={{ fontFamily: 'monospace', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                                {url}
                              </div>
                            </div>
                            <button
                              className="btn btn-ghost btn-sm text-danger"
                              style={{ padding: '4px 8px' }}
                              onClick={() => {
                                const targetId = projectIdByName.get(depName) ?? `external:${depName}`;
                                setSelectedEdgeId(`debug:${selectedProject.id}:${depName}`);
                                setSelectedNodeId(null);
                                setEdges((prev) => prev.filter((e) => !(e.source === selectedProject.id && e.target === targetId && e.id === `debug:${selectedProject.id}:${depName}`)));
                                setDraftByProjectId((prev) => {
                                  const next = { ...prev };
                                  const current = next[selectedProject.id] ?? {};
                                  if (!(depName in current)) return prev;
                                  const updated = { ...current };
                                  delete updated[depName];
                                  next[selectedProject.id] = updated;
                                  return next;
                                });
                                setDirtyProjectIds((prev) => new Set(prev).add(selectedProject.id));
                                setTouchedProjectIds((prev) => {
                                  const next = new Set(prev);
                                  next.add(selectedProject.id);
                                  const targetProjectId = projectIdByName.get(depName);
                                  if (targetProjectId) next.add(targetProjectId);
                                  return next;
                                });
                              }}
                            >
                              移除
                            </button>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  <div className="text-xs text-muted">
                    提示：也可以在图中从该节点拖出连线到目标节点来添加依赖。
                  </div>
                </div>
              ) : (
                <div className="text-xs text-muted" style={{ marginTop: 10 }}>
                  点击一个节点或连线以查看/编辑详情。
                </div>
              )}
            </div>

            <div className="dependency-graph__side-card">
              <div className="text-xs font-semibold text-secondary" style={{ textTransform: 'uppercase', letterSpacing: '0.06em' }}>
                快捷操作
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                <div className="text-xs text-muted">
                  已修改项目：{dirtyCount} 个
                </div>
                <button
                  className="btn btn-secondary"
                  disabled={involvedRunningProjectIds.length === 0}
                  title={involvedRunningProjectIds.length === 0 ? '无涉及运行中项目' : `重启 ${involvedRunningProjectIds.length} 个涉及项目`}
                  onClick={async () => {
                    if (involvedRunningProjectIds.length === 0) return;
                    if (!confirm(`将重启 ${involvedRunningProjectIds.length} 个正在运行的项目，是否继续？`)) return;
                    await onRestartRunningProjects(involvedRunningProjectIds);
                  }}
                >
                  重启涉及运行中项目（{involvedRunningProjectIds.length}）
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

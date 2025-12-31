import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { AppSettings, GitPullResult, GitStatus, ProjectInfo } from '../types';
import { getGitStatus, gitFetch, gitPull, isGitRepo } from '../services/tauri';

const GIT_NOT_INSTALLED = 'GIT_NOT_INSTALLED';
const NOT_GIT_REPO = 'NOT_GIT_REPO';

type GitBusyState = {
  fetching: boolean;
  pulling: boolean;
};

function getOrDefaultBusy(map: Map<string, GitBusyState>, projectId: string): GitBusyState {
  return map.get(projectId) ?? { fetching: false, pulling: false };
}

async function notifyGitUpdate(projectName: string, behindCount: number) {
  if (!('Notification' in window)) return;
  if (Notification.permission === 'default') {
    try {
      await Notification.requestPermission();
    } catch {
      return;
    }
  }
  if (Notification.permission !== 'granted') return;
  try {
    new Notification('Git 更新', { body: `${projectName} 有 ${behindCount} 个可拉取更新` });
  } catch {
    // ignore
  }
}

export function useGitStatus(projects: ProjectInfo[], settings: AppSettings) {
  const [gitStatuses, setGitStatuses] = useState<Map<string, GitStatus | null>>(new Map());
  const [gitBusyByProjectId, setGitBusyByProjectId] = useState<Map<string, GitBusyState>>(new Map());
  const [gitDisabledReason, setGitDisabledReason] = useState<string | null>(null);

  const prevBehindByProjectId = useRef<Map<string, number>>(new Map());

  // Only re-initialize / refresh git status when the git-relevant identity of projects changes.
  // (e.g. bulk-start enabled toggle should not cause a git refresh.)
  const projectsKey = useMemo(() => {
    return projects
      .map((p) => `${p.id}::${p.path}::${p.is_valid ? 1 : 0}`)
      .sort()
      .join('|');
  }, [projects]);

  const repoProjects = useMemo(() => {
    return projects.filter((p) => p.is_valid);
  }, [projectsKey]);

  const setBusy = useCallback((projectId: string, updates: Partial<GitBusyState>) => {
    setGitBusyByProjectId((prev) => {
      const next = new Map(prev);
      const current = getOrDefaultBusy(next, projectId);
      next.set(projectId, { ...current, ...updates });
      return next;
    });
  }, []);

  const refreshProject = useCallback(
    async (project: ProjectInfo) => {
      if (gitDisabledReason) return;
      const isRepo = await isGitRepo(project.path);
      if (!isRepo) {
        setGitStatuses((prev) => new Map(prev).set(project.id, null));
        return;
      }
      try {
        const status = await getGitStatus(project.path);
        setGitStatuses((prev) => new Map(prev).set(project.id, status));
        prevBehindByProjectId.current.set(project.id, status.behind_count);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (message.includes(GIT_NOT_INSTALLED)) {
          setGitDisabledReason('Git 未安装，已禁用 Git 功能');
        }
        if (message.includes(NOT_GIT_REPO)) {
          setGitStatuses((prev) => new Map(prev).set(project.id, null));
        }
      }
    },
    [gitDisabledReason]
  );

  const refreshAll = useCallback(async () => {
    if (gitDisabledReason) return;
    await Promise.allSettled(repoProjects.map((p) => refreshProject(p)));
  }, [gitDisabledReason, repoProjects, refreshProject]);

  const fetchProject = useCallback(
    async (project: ProjectInfo) => {
      if (gitDisabledReason) return;
      setBusy(project.id, { fetching: true });
      try {
        const isRepo = await isGitRepo(project.path);
        if (!isRepo) {
          setGitStatuses((prev) => new Map(prev).set(project.id, null));
          return;
        }
        const prevBehind = prevBehindByProjectId.current.get(project.id) ?? 0;
        const status = await gitFetch(project.path);
        setGitStatuses((prev) => new Map(prev).set(project.id, status));
        prevBehindByProjectId.current.set(project.id, status.behind_count);

        if (settings.gitNotificationsEnabled && status.behind_count > prevBehind) {
          await notifyGitUpdate(project.name, status.behind_count);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (message.includes(GIT_NOT_INSTALLED)) {
          setGitDisabledReason('Git 未安装，已禁用 Git 功能');
        }
        throw err;
      } finally {
        setBusy(project.id, { fetching: false });
      }
    },
    [gitDisabledReason, setBusy, settings.gitNotificationsEnabled]
  );

  const pullProject = useCallback(
    async (project: ProjectInfo): Promise<GitPullResult | null> => {
      if (gitDisabledReason) return null;
      setBusy(project.id, { pulling: true });
      try {
        const result = await gitPull(project.path);
        setGitStatuses((prev) => new Map(prev).set(project.id, result.status));
        prevBehindByProjectId.current.set(project.id, result.status.behind_count);
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (message.includes(GIT_NOT_INSTALLED)) {
          setGitDisabledReason('Git 未安装，已禁用 Git 功能');
        }
        throw err;
      } finally {
        setBusy(project.id, { pulling: false });
      }
    },
    [gitDisabledReason, setBusy]
  );

  useEffect(() => {
    const currentIds = new Set(projects.map((p) => p.id));

    setGitStatuses((prev) => {
      if (prev.size === 0) return prev;
      const next = new Map<string, GitStatus | null>();
      prev.forEach((value, key) => {
        if (currentIds.has(key)) next.set(key, value);
      });
      return next;
    });

    setGitBusyByProjectId((prev) => {
      if (prev.size === 0) return prev;
      const next = new Map<string, GitBusyState>();
      prev.forEach((value, key) => {
        if (currentIds.has(key)) next.set(key, value);
      });
      return next;
    });

    prevBehindByProjectId.current = new Map(
      Array.from(prevBehindByProjectId.current.entries()).filter(([id]) => currentIds.has(id))
    );

    void refreshAll();
  }, [projectsKey, refreshAll]);

  useEffect(() => {
    if (gitDisabledReason) return;
    const minutes = settings.gitFetchIntervalMinutes;
    if (!minutes || minutes <= 0) return;

    const id = setInterval(() => {
      void Promise.allSettled(repoProjects.map((p) => fetchProject(p)));
    }, minutes * 60 * 1000);

    return () => clearInterval(id);
  }, [fetchProject, gitDisabledReason, repoProjects, settings.gitFetchIntervalMinutes]);

  return {
    gitStatuses,
    gitBusyByProjectId,
    gitDisabledReason,
    refreshAll,
    refreshProject,
    fetchProject,
    pullProject,
  };
}

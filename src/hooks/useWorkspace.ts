import { useState, useCallback } from 'react';
import { open } from '@tauri-apps/api/dialog';
import {
  createWorkspace,
  scanWorkspaceProjects,
  saveWorkspace,
  resolvePortConflicts,
  addWorkspaceFolder,
  removeWorkspaceFolder,
} from '../services/tauri';
import type { Workspace, PortChange } from '../types';

export function useWorkspace() {
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 选择目录并创建工作区
  const selectAndCreateWorkspace = useCallback(async (name: string) => {
    try {
      setLoading(true);
      setError(null);

      // 打开目录选择对话框（支持多选）
      const selected = await open({
        directory: true,
        multiple: true,  // 支持多选
        title: '选择工作区文件夹（可多选）',
      });

      if (!selected || (Array.isArray(selected) && selected.length === 0)) {
        setLoading(false);
        return null;
      }

      // 确保是数组格式
      const folders = Array.isArray(selected) ? selected : [selected];

      // 创建工作区（会自动扫描项目）
      const newWorkspace = await createWorkspace(name, folders);
      setWorkspace(newWorkspace);
      setLoading(false);
      return newWorkspace;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
      return null;
    }
  }, []);

  // 重新扫描工作区项目
  const rescanProjects = useCallback(async () => {
    if (!workspace) return;

    try {
      setLoading(true);
      setError(null);

      const projects = await scanWorkspaceProjects(workspace.folders);
      const updatedWorkspace = {
        ...workspace,
        projects,
        last_modified: new Date().toISOString(),
      };

      setWorkspace(updatedWorkspace);
      await saveWorkspace(updatedWorkspace);
      setLoading(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
    }
  }, [workspace]);

  // 添加文件夹到工作区
  const addFolder = useCallback(async () => {
    if (!workspace) return;

    try {
      setLoading(true);
      setError(null);

      // 打开目录选择对话框
      const selected = await open({
        directory: true,
        multiple: false,
        title: '选择要添加的文件夹',
      });

      if (!selected || Array.isArray(selected)) {
        setLoading(false);
        return;
      }

      // 添加文件夹（会自动重新扫描）
      const updatedWorkspace = await addWorkspaceFolder(workspace, selected);
      setWorkspace(updatedWorkspace);
      setLoading(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
    }
  }, [workspace]);

  // 从工作区移除文件夹
  const removeFolder = useCallback(async (folderPath: string) => {
    if (!workspace) return;

    try {
      setLoading(true);
      setError(null);

      // 移除文件夹（会自动重新扫描）
      const updatedWorkspace = await removeWorkspaceFolder(workspace, folderPath);
      setWorkspace(updatedWorkspace);
      setLoading(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
    }
  }, [workspace]);

  // 解决端口冲突
  const resolveConflicts = useCallback(async (): Promise<PortChange[]> => {
    if (!workspace) return [];

    try {
      setLoading(true);
      setError(null);

      const [updatedProjects, changes] = await resolvePortConflicts(
        workspace.id,
        workspace.projects,
        workspace.settings.port_range_start,
        workspace.settings.port_range_end
      );

      if (changes.length > 0) {
        const updatedWorkspace = {
          ...workspace,
          projects: updatedProjects,
          last_modified: new Date().toISOString(),
        };

        setWorkspace(updatedWorkspace);
        await saveWorkspace(updatedWorkspace);
      }

      setLoading(false);
      return changes;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
      return [];
    }
  }, [workspace]);

  // 更新工作区
  const updateWorkspace = useCallback(async (updates: Partial<Workspace>) => {
    if (!workspace) return;

    const updatedWorkspace = {
      ...workspace,
      ...updates,
      last_modified: new Date().toISOString(),
    };

    setWorkspace(updatedWorkspace);
    await saveWorkspace(updatedWorkspace);
  }, [workspace]);

  return {
    workspace,
    loading,
    error,
    selectAndCreateWorkspace,
    rescanProjects,
    resolveConflicts,
    updateWorkspace,
    addFolder,
    removeFolder,
    setError,
    setWorkspace,
  };
}

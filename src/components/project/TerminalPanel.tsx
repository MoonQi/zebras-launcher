import { useEffect, useMemo, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { TerminalLogMessage, TerminalSession } from '../../types';
import {
  closeTerminalSession,
  createTerminalSession,
  getTerminalSessions,
  killTerminalSession,
  runTerminalCommand,
} from '../../services/tauri';

interface TerminalPanelProps {
  projectId: string;
  projectPath: string;
}

const QUICK_COMMANDS: Array<{ label: string; command: string }> = [
  { label: 'npm i', command: 'npm install' },
  { label: 'pnpm i', command: 'pnpm install' },
  { label: 'npm run deploy', command: 'npm run deploy' },
];

export function TerminalPanel({ projectId, projectPath }: TerminalPanelProps) {
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [commandBySessionId, setCommandBySessionId] = useState<Map<string, string>>(new Map());
  const [logsBySessionId, setLogsBySessionId] = useState<Map<string, TerminalLogMessage[]>>(new Map());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const logContainerRef = useRef<HTMLDivElement>(null);

  const activeSession = useMemo(
    () => sessions.find((s) => s.session_id === activeSessionId) ?? null,
    [activeSessionId, sessions]
  );

  const activeCommand = useMemo(() => {
    if (!activeSessionId) return '';
    return commandBySessionId.get(activeSessionId) ?? '';
  }, [activeSessionId, commandBySessionId]);

  const activeLogs = useMemo(() => {
    if (!activeSessionId) return [];
    return logsBySessionId.get(activeSessionId) ?? [];
  }, [activeSessionId, logsBySessionId]);

  const refreshSessions = async () => {
    const next = await getTerminalSessions(projectId);
    setSessions(next);
    if (next.length > 0 && (!activeSessionId || !next.some((s) => s.session_id === activeSessionId))) {
      setActiveSessionId(next[0].session_id);
    }
  };

  useEffect(() => {
    void (async () => {
      try {
        setBusy(true);
        setError(null);
        const existing = await getTerminalSessions(projectId);
        if (existing.length > 0) {
          setSessions(existing);
          setActiveSessionId(existing[0].session_id);
          return;
        }
        const created = await createTerminalSession(projectId);
        setSessions([created]);
        setActiveSessionId(created.session_id);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setBusy(false);
      }
    })();
  }, [projectId]);

  useEffect(() => {
    const unlisten = listen<TerminalLogMessage>('terminal_log', (event) => {
      const payload = event.payload;
      if (payload.project_id !== projectId) return;
      setLogsBySessionId((prev) => {
        const next = new Map(prev);
        const list = next.get(payload.session_id) ?? [];
        next.set(payload.session_id, [...list, payload]);
        return next;
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [projectId]);

  useEffect(() => {
    const container = logContainerRef.current;
    if (!container) return;
    container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
  }, [activeLogs.length]);

  const handleCreate = async () => {
    try {
      setBusy(true);
      setError(null);
      const created = await createTerminalSession(projectId);
      setSessions((prev) => [...prev, created]);
      setActiveSessionId(created.session_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleClose = async (sessionId: string) => {
    try {
      setBusy(true);
      setError(null);
      await closeTerminalSession(sessionId);
      setSessions((prev) => prev.filter((s) => s.session_id !== sessionId));
      setLogsBySessionId((prev) => {
        const next = new Map(prev);
        next.delete(sessionId);
        return next;
      });
      setCommandBySessionId((prev) => {
        const next = new Map(prev);
        next.delete(sessionId);
        return next;
      });
      await refreshSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRun = async (command: string) => {
    if (!activeSessionId) return;
    try {
      setBusy(true);
      setError(null);
      await runTerminalCommand(activeSessionId, projectPath, command);
      await refreshSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleKill = async () => {
    if (!activeSessionId) return;
    try {
      setBusy(true);
      setError(null);
      await killTerminalSession(activeSessionId);
      await refreshSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleClear = () => {
    if (!activeSessionId) return;
    setLogsBySessionId((prev) => new Map(prev).set(activeSessionId, []));
  };

  const canCreateMore = sessions.length < 3;

  return (
    <div
      style={{
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
        border: '1px solid var(--color-border)',
        backgroundColor: 'rgba(0,0,0,0.2)',
      }}
    >
      <div className="flex justify-between items-center" style={{ padding: '8px 10px' }}>
        <div className="text-xs font-semibold text-primary">终端</div>
        <div className="text-xs text-muted">{busy ? '处理中...' : ''}</div>
      </div>

      <div
        className="flex items-center gap-xs"
        style={{ padding: '0 10px 10px', borderBottom: '1px solid rgba(255,255,255,0.05)' }}
      >
        {sessions.map((s, idx) => (
          <button
            key={s.session_id}
            onClick={() => setActiveSessionId(s.session_id)}
            className={`btn ${s.session_id === activeSessionId ? 'btn-primary' : 'btn-secondary'}`}
            style={{
              padding: '0.25rem 0.5rem',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              opacity: busy ? 0.8 : 1,
            }}
            title={s.command ?? `Tab ${idx + 1}`}
            disabled={busy}
          >
            <span>{`Tab ${idx + 1}`}</span>
            <span className="text-xs" style={{ opacity: 0.8 }}>
              {s.status}
            </span>
            {sessions.length > 1 && (
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  void handleClose(s.session_id);
                }}
                style={{
                  padding: '0 4px',
                  borderRadius: 4,
                  opacity: 0.9,
                }}
                title="关闭"
              >
                ×
              </span>
            )}
          </button>
        ))}

        <button
          onClick={() => void handleCreate()}
          className={`btn ${canCreateMore ? 'btn-secondary' : 'btn-ghost'}`}
          disabled={!canCreateMore || busy}
          style={{ padding: '0.25rem 0.5rem' }}
          title={canCreateMore ? '新增终端' : '已达到上限（3 个）'}
        >
          +
        </button>
      </div>

      <div className="flex gap-xs flex-wrap" style={{ padding: '10px' }}>
        {QUICK_COMMANDS.map((c) => (
          <button
            key={c.label}
            className="btn btn-secondary"
            disabled={busy || !activeSessionId}
            style={{ padding: '0.35rem 0.75rem' }}
            onClick={() => {
              if (!activeSessionId) return;
              setCommandBySessionId((prev) => new Map(prev).set(activeSessionId, c.command));
              void handleRun(c.command);
            }}
          >
            {c.label}
          </button>
        ))}
      </div>

      <div
        className="flex gap-xs items-center"
        style={{ padding: '0 10px 10px', borderBottom: '1px solid rgba(255,255,255,0.05)' }}
      >
        <input
          className="input"
          value={activeCommand}
          placeholder="输入命令（在项目目录执行）"
          disabled={busy || !activeSessionId}
          onChange={(e) => {
            if (!activeSessionId) return;
            const next = e.target.value;
            setCommandBySessionId((prev) => new Map(prev).set(activeSessionId, next));
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              void handleRun(activeCommand);
            }
          }}
          style={{ flex: 1, fontFamily: 'monospace' }}
        />
        <button className="btn btn-primary" disabled={busy || !activeSessionId} onClick={() => void handleRun(activeCommand)}>
          Run
        </button>
        <button
          className="btn btn-danger"
          disabled={busy || !activeSessionId || activeSession?.status !== 'running'}
          onClick={() => void handleKill()}
        >
          Kill
        </button>
        <button className="btn btn-secondary" disabled={busy || !activeSessionId} onClick={handleClear}>
          Clear
        </button>
      </div>

      {error && (
        <div className="text-xs text-danger" style={{ padding: '8px 10px' }}>
          {error}
        </div>
      )}

      <div
        ref={logContainerRef}
        style={{
          padding: '10px',
          fontFamily: 'monospace',
          fontSize: '11px',
          lineHeight: '1.5',
          height: '220px',
          overflowY: 'auto',
          color: '#e5e7eb',
          backgroundColor: '#0c0c0c',
        }}
      >
        {activeLogs.length === 0 ? (
          <div style={{ color: '#6b7280', fontStyle: 'italic' }}>暂无输出...</div>
        ) : (
          activeLogs.map((log, idx) => (
            <div
              key={idx}
              style={{
                color: log.stream === 'stderr' ? '#f87171' : '#4ade80',
                wordBreak: 'break-all',
                marginBottom: '2px',
              }}
            >
              {log.message}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

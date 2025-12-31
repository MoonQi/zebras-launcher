import type { AppSettings } from '../../types';

interface SettingsPanelProps {
  settings: AppSettings;
  onChange: (updates: Partial<AppSettings>) => void;
  onReset: () => void;
  onClose: () => void;
}

export function SettingsPanel({ settings, onChange, onReset, onClose }: SettingsPanelProps) {
  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0,0,0,0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        padding: '24px',
      }}
      onClick={onClose}
    >
      <div className="card" style={{ width: '520px', maxWidth: '100%' }} onClick={(e) => e.stopPropagation()}>
        <div className="flex justify-between items-center mb-md">
          <h2 className="m-0">设置</h2>
          <button className="btn btn-ghost" onClick={onClose} style={{ padding: '0.25rem 0.5rem' }}>
            ×
          </button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <label className="flex items-center gap-sm text-sm">
            <input
              type="checkbox"
              checked={settings.gitNotificationsEnabled}
              onChange={(e) => onChange({ gitNotificationsEnabled: e.target.checked })}
              style={{ accentColor: 'var(--color-primary)' }}
            />
            <span>启用 Git 更新通知</span>
          </label>

          <div>
            <label className="block mb-sm text-secondary text-sm">Git Fetch 间隔（分钟）</label>
            <input
              type="number"
              className="input"
              min={1}
              value={settings.gitFetchIntervalMinutes}
              onChange={(e) => {
                const next = Number(e.target.value);
                onChange({ gitFetchIntervalMinutes: Number.isFinite(next) && next > 0 ? next : 15 });
              }}
            />
            <div className="text-xs text-muted mt-xs">默认每 15 分钟自动 fetch 并检查远程更新。</div>
          </div>
        </div>

        <div className="flex gap-sm mt-lg">
          <button className="btn btn-secondary" onClick={onReset}>
            恢复默认
          </button>
          <div className="flex-1" />
          <button className="btn btn-primary" onClick={onClose}>
            完成
          </button>
        </div>
      </div>
    </div>
  );
}


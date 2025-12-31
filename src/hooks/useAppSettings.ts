import { useCallback, useEffect, useState } from 'react';
import type { AppSettings } from '../types';

const STORAGE_KEY = 'zebras_launcher_settings';

const DEFAULT_SETTINGS: AppSettings = {
  gitFetchIntervalMinutes: 15,
  gitNotificationsEnabled: true,
};

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<AppSettings>;
    return {
      gitFetchIntervalMinutes:
        typeof parsed.gitFetchIntervalMinutes === 'number' && parsed.gitFetchIntervalMinutes > 0
          ? parsed.gitFetchIntervalMinutes
          : DEFAULT_SETTINGS.gitFetchIntervalMinutes,
      gitNotificationsEnabled:
        typeof parsed.gitNotificationsEnabled === 'boolean'
          ? parsed.gitNotificationsEnabled
          : DEFAULT_SETTINGS.gitNotificationsEnabled,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings>(() => loadSettings());

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }, [settings]);

  const updateSettings = useCallback((updates: Partial<AppSettings>) => {
    setSettings((prev) => ({ ...prev, ...updates }));
  }, []);

  const resetSettings = useCallback(() => {
    setSettings(DEFAULT_SETTINGS);
  }, []);

  return { settings, updateSettings, resetSettings };
}


import * as api from '../api/backend';
import { errorMessage } from '../api/backend';
import type { Settings } from '../api/types';
import { setLanguage, t } from '../i18n';
import { ui } from './ui.svelte';

export function fallbackSettings(): Settings {
  return {
    version: 1, roots: [], excludeGlobs: [], includeExtensions: [], excludeExtensions: [],
    maxFileSizeBytes: 50 * 1024 * 1024, storeContent: true, indexHidden: false, followLinks: false,
    watchChanges: true, workerThreads: 0, writerMemoryMb: 256, pageSize: 50, uiLanguage: 'ru', theme: 'system',
    reindexOnStart: false, extractPdf: true
  };
}

class SettingsState {
  value = $state<Settings>(fallbackSettings());
  loaded = $state(false);
  saving = $state(false);
  loadError = $state<string | null>(null);

  get hasRoots(): boolean { return this.value.roots.length > 0; }
  get enabledRoots(): string[] { return this.value.roots.filter((r) => r.enabled).map((r) => r.path); }

  apply(s: Settings): void {
    this.value = s;
    setLanguage(s.uiLanguage);
    ui.applyTheme(s.theme);
  }

  async load(): Promise<void> {
    try {
      const s = await api.getSettings();
      this.apply(s);
      this.loadError = null;
    } catch (e) {
      this.loadError = errorMessage(e);
      ui.error(t('toast.error'), this.loadError);
      // Keep the fallback so the UI still renders.
      setLanguage(this.value.uiLanguage);
      ui.applyTheme(this.value.theme);
    } finally {
      this.loaded = true;
    }
  }

  /** Saves a full settings object; returns true on success. */
  async save(next: Settings): Promise<boolean> {
    this.saving = true;
    try {
      const saved = await api.saveSettings(next);
      this.apply(saved);
      return true;
    } catch (e) {
      ui.error(t('toast.error'), errorMessage(e));
      return false;
    } finally {
      this.saving = false;
    }
  }

  async patch(partial: Partial<Settings>): Promise<boolean> {
    return this.save({ ...this.value, ...partial });
  }

  /** Adds roots (deduplicated, case-insensitive) and saves. Returns the number of new roots. */
  async addRoots(paths: string[]): Promise<number> {
    const existing = new Set(this.value.roots.map((r) => r.path.toLowerCase()));
    const fresh = paths.filter((p) => p && !existing.has(p.toLowerCase()));
    if (fresh.length === 0) return 0;
    const ok = await this.save({ ...this.value, roots: [...this.value.roots, ...fresh.map((path) => ({ path, enabled: true }))] });
    return ok ? fresh.length : 0;
  }
}

export const settings = new SettingsState();

import * as api from '../api/backend';
import { errorMessage } from '../api/backend';
import type { IndexProgress, IndexStats } from '../api/types';
import { t } from '../i18n';
import { formatNumber } from '../utils/format';
import { ui } from './ui.svelte';
import { settings } from './settings.svelte';

export function idleProgress(): IndexProgress {
  return {
    running: false, phase: 'idle', scanned: 0, queued: 0, indexed: 0, unchanged: 0, nameOnly: 0, failed: 0,
    deleted: 0, bytes: 0, current: null, error: null, elapsedMs: 0
  };
}

class IndexingState {
  progress = $state<IndexProgress>(idleProgress());
  stats = $state<IndexStats | null>(null);
  statsError = $state<string | null>(null);
  /** Callbacks fired when an indexing run finishes (e.g. re-run the search). */
  #finishedHandlers = new Set<() => void>();
  #unsubs: Array<() => void> = [];
  #statsTimer: ReturnType<typeof setInterval> | null = null;

  get running(): boolean { return this.progress.running; }
  get hasIndex(): boolean { return (this.stats?.documents ?? 0) > 0; }

  /** Fraction 0..1 when determinable, otherwise null (indeterminate). */
  get fraction(): number | null {
    const p = this.progress;
    if (!p.running) return null;
    if (p.phase === 'extracting' && p.queued > 0) return Math.min(1, p.indexed / p.queued);
    if (p.phase === 'cleaning' || p.phase === 'committing') return 1;
    return null;
  }

  /** Bytes per second during extraction. */
  get speed(): number {
    const p = this.progress;
    return p.elapsedMs > 500 ? (p.bytes * 1000) / p.elapsedMs : 0;
  }

  onFinished(cb: () => void): () => void {
    this.#finishedHandlers.add(cb);
    return () => this.#finishedHandlers.delete(cb);
  }

  init(): void {
    this.#unsubs.push(api.onIndexProgress((p) => { this.progress = p; }));
    this.#unsubs.push(api.onIndexFinished((p) => {
      this.progress = p;
      if (p.phase === 'done') ui.success(t('index.done', { count: formatNumber(p.indexed, settings.value.uiLanguage) }));
      else if (p.phase === 'cancelled') ui.info(t('index.cancelled'));
      else if (p.phase === 'failed') ui.error(t('index.failed', { error: p.error ?? '' }));
      void this.refreshStats();
      for (const cb of this.#finishedHandlers) cb();
    }));
    void this.refreshProgress();
    void this.refreshStats();
    // Stats are cheap; keep the "updated N min ago" label fresh and catch external changes.
    this.#statsTimer = setInterval(() => { if (!this.progress.running) void this.refreshStats(); }, 60_000);
  }

  destroy(): void {
    for (const u of this.#unsubs) u();
    this.#unsubs = [];
    if (this.#statsTimer) clearInterval(this.#statsTimer);
  }

  async refreshProgress(): Promise<void> {
    try { this.progress = await api.getProgress(); } catch { /* keep the last known state */ }
  }

  async refreshStats(): Promise<void> {
    try {
      this.stats = await api.getStats();
      this.statsError = null;
    } catch (e) {
      this.statsError = errorMessage(e);
    }
  }

  async start(full: boolean): Promise<boolean> {
    try {
      await api.startIndexing(full);
      this.progress = { ...idleProgress(), running: true, phase: 'scanning' };
      ui.info(t('index.started'));
      return true;
    } catch (e) {
      ui.error(t('toast.error'), errorMessage(e));
      return false;
    }
  }

  async cancel(): Promise<void> {
    try { await api.cancelIndexing(); } catch (e) { ui.error(t('toast.error'), errorMessage(e)); }
  }

  async clear(): Promise<boolean> {
    const ok = await ui.ask(t('common.confirm'), t('index.clear.confirm'), t('index.clear'), true);
    if (!ok) return false;
    try {
      await api.clearIndex();
      ui.success(t('index.cleared'));
      await this.refreshStats();
      for (const cb of this.#finishedHandlers) cb();
      return true;
    } catch (e) {
      ui.error(t('toast.error'), errorMessage(e));
      return false;
    }
  }
}

export const indexing = new IndexingState();

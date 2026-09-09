import type {
  AppInfo, DriveInfo, IndexProgress, IndexStats, Preview, SearchMode, SearchRequest, SearchResponse, Settings
} from './types';

export interface Backend {
  getSettings(): Promise<Settings>;
  saveSettings(settings: Settings): Promise<Settings>;
  listDrives(): Promise<DriveInfo[]>;
  pickFolders(): Promise<string[]>;
  startIndexing(full: boolean): Promise<void>;
  cancelIndexing(): Promise<void>;
  getProgress(): Promise<IndexProgress>;
  search(request: SearchRequest): Promise<SearchResponse>;
  getPreview(path: string, query: string, mode: SearchMode): Promise<Preview>;
  getStats(): Promise<IndexStats>;
  clearIndex(): Promise<void>;
  openFile(path: string): Promise<void>;
  revealInExplorer(path: string): Promise<void>;
  openWith(path: string): Promise<void>;
  copyText(text: string): Promise<void>;
  analyzeText(text: string, mode: SearchMode): Promise<string[]>;
  getAppInfo(): Promise<AppInfo>;
  openLogsFolder(): Promise<void>;
  openDataFolder(): Promise<void>;
  onIndexProgress(cb: (p: IndexProgress) => void): () => void;
  onIndexFinished(cb: (p: IndexProgress) => void): () => void;
}

export const isTauri: boolean = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Token injected by the embedded HTTP server (browser mode, no WebView2). */
const browserToken: string | null =
  typeof window !== 'undefined' && typeof (window as { __QIDIR_TOKEN__?: unknown }).__QIDIR_TOKEN__ === 'string'
    ? (window as unknown as { __QIDIR_TOKEN__: string }).__QIDIR_TOKEN__
    : null;
export const isBrowserMode: boolean = !isTauri && browserToken !== null;

/** Normalizes a rejected command into a plain Error with a readable message. */
export function errorMessage(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') {
    return (e as { message: string }).message;
  }
  try { return JSON.stringify(e); } catch { return String(e); }
}

function createTauriBackend(): Backend {
  // Lazy imports keep the plain-browser bundle from touching Tauri internals at startup.
  const core = import('@tauri-apps/api/core');
  const event = import('@tauri-apps/api/event');

  async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await core;
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      throw new Error(errorMessage(e));
    }
  }

  function subscribe<T>(name: string, cb: (payload: T) => void): () => void {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    event.then(({ listen }) => listen<T>(name, (ev) => cb(ev.payload))).then((fn) => {
      if (cancelled) fn(); else unlisten = fn;
    }).catch(() => { /* listener setup failed; nothing to do */ });
    return () => { cancelled = true; unlisten?.(); };
  }

  return {
    getSettings: () => invoke<Settings>('get_settings'),
    saveSettings: (settings) => invoke<Settings>('save_settings', { settings }),
    listDrives: () => invoke<DriveInfo[]>('list_drives'),
    pickFolders: () => invoke<string[]>('pick_folders'),
    startIndexing: (full) => invoke<void>('start_indexing', { full }),
    cancelIndexing: () => invoke<void>('cancel_indexing'),
    getProgress: () => invoke<IndexProgress>('get_progress'),
    search: (request) => invoke<SearchResponse>('search', { request }),
    getPreview: (path, query, mode) => invoke<Preview>('get_preview', { path, query, mode }),
    getStats: () => invoke<IndexStats>('get_stats'),
    clearIndex: () => invoke<void>('clear_index'),
    openFile: (path) => invoke<void>('open_file', { path }),
    revealInExplorer: (path) => invoke<void>('reveal_in_explorer', { path }),
    openWith: (path) => invoke<void>('open_with', { path }),
    copyText: (text) => invoke<void>('copy_text', { text }),
    analyzeText: (text, mode) => invoke<string[]>('analyze_text', { text, mode }),
    getAppInfo: () => invoke<AppInfo>('get_app_info'),
    openLogsFolder: () => invoke<void>('open_logs_folder'),
    openDataFolder: () => invoke<void>('open_data_folder'),
    onIndexProgress: (cb) => subscribe<IndexProgress>('index-progress', cb),
    onIndexFinished: (cb) => subscribe<IndexProgress>('index-finished', cb)
  };
}

/** Transport for the embedded local server: same command names, JSON over POST. */
function createHttpBackend(token: string): Backend {
  async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    let res: Response;
    try {
      res = await fetch(`/api/${cmd}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-QIDIR-Token': token },
        body: JSON.stringify(args ?? {})
      });
    } catch (e) {
      throw new Error(`QIDIR не отвечает / QIDIR is not responding: ${errorMessage(e)}`);
    }
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = (await res.json()) as { ok: boolean; result?: T; error?: string };
    if (!data.ok) throw new Error(data.error ?? 'unknown error');
    return data.result as T;
  }

  // Progress events are polled while someone listens; the polling doubles
  // as the heartbeat that keeps the server alive.
  const progressListeners = new Set<(p: IndexProgress) => void>();
  const finishedListeners = new Set<(p: IndexProgress) => void>();
  let timer: ReturnType<typeof setInterval> | null = null;
  let wasRunning = false;
  function ensurePolling() {
    if (timer) return;
    timer = setInterval(async () => {
      try {
        const p = await call<IndexProgress>('get_progress');
        if (p.running) {
          progressListeners.forEach((cb) => cb(p));
          wasRunning = true;
        } else if (wasRunning) {
          wasRunning = false;
          progressListeners.forEach((cb) => cb(p));
          finishedListeners.forEach((cb) => cb(p));
        }
      } catch { /* server gone; the next user action will surface the error */ }
    }, 700);
  }
  function subscribe(set: Set<(p: IndexProgress) => void>, cb: (p: IndexProgress) => void): () => void {
    set.add(cb);
    ensurePolling();
    return () => { set.delete(cb); };
  }
  // Heartbeat even without listeners.
  setInterval(() => { call('ping').catch(() => undefined); }, 20_000);
  window.addEventListener('beforeunload', () => {
    // Best effort: tell the server the tab is gone so it can exit sooner.
    try { navigator.sendBeacon?.('/api/ping', '{}'); } catch { /* ignore */ }
  });

  return {
    getSettings: () => call<Settings>('get_settings'),
    saveSettings: (settings) => call<Settings>('save_settings', { settings }),
    listDrives: () => call<DriveInfo[]>('list_drives'),
    pickFolders: () => call<string[]>('pick_folders'),
    startIndexing: (full) => call<void>('start_indexing', { full }),
    cancelIndexing: () => call<void>('cancel_indexing'),
    getProgress: () => call<IndexProgress>('get_progress'),
    search: (request) => call<SearchResponse>('search', { request }),
    getPreview: (path, query, mode) => call<Preview>('get_preview', { path, query, mode }),
    getStats: () => call<IndexStats>('get_stats'),
    clearIndex: () => call<void>('clear_index'),
    openFile: (path) => call<void>('open_file', { path }),
    revealInExplorer: (path) => call<void>('reveal_in_explorer', { path }),
    openWith: (path) => call<void>('open_with', { path }),
    copyText: async (text) => {
      if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(text); return; }
      const ta = document.createElement('textarea');
      ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0';
      document.body.appendChild(ta); ta.select();
      try { document.execCommand('copy'); } finally { ta.remove(); }
    },
    analyzeText: (text, mode) => call<string[]>('analyze_text', { text, mode }),
    getAppInfo: () => call<AppInfo>('get_app_info'),
    openLogsFolder: () => call<void>('open_logs_folder'),
    openDataFolder: () => call<void>('open_data_folder'),
    onIndexProgress: (cb) => subscribe(progressListeners, cb),
    onIndexFinished: (cb) => subscribe(finishedListeners, cb)
  };
}

/** Browser mode only: ask the local server to exit. */
export async function shutdownServer(): Promise<void> {
  if (!browserToken) return;
  await fetch('/api/shutdown', { method: 'POST', headers: { 'X-QIDIR-Token': browserToken }, body: '{}' }).catch(() => undefined);
}

let backendInstance: Backend | null = null;
let mockPromise: Promise<Backend> | null = null;

async function resolveBackend(): Promise<Backend> {
  if (backendInstance) return backendInstance;
  if (isTauri) {
    backendInstance = createTauriBackend();
    return backendInstance;
  }
  if (browserToken) {
    backendInstance = createHttpBackend(browserToken);
    return backendInstance;
  }
  if (!mockPromise) {
    mockPromise = import('./mock').then((m) => {
      backendInstance = m.createMockBackend();
      return backendInstance;
    });
  }
  return mockPromise;
}

function call<K extends keyof Backend>(key: K): Backend[K] {
  return ((...args: unknown[]) =>
    resolveBackend().then((b) => (b[key] as (...a: unknown[]) => unknown)(...args))) as Backend[K];
}

export const getSettings = call('getSettings');
export const saveSettings = call('saveSettings');
export const listDrives = call('listDrives');
export const pickFolders = call('pickFolders');
export const startIndexing = call('startIndexing');
export const cancelIndexing = call('cancelIndexing');
export const getProgress = call('getProgress');
export const search = call('search');
export const getPreview = call('getPreview');
export const getStats = call('getStats');
export const clearIndex = call('clearIndex');
export const openFile = call('openFile');
export const revealInExplorer = call('revealInExplorer');
export const openWith = call('openWith');
export const copyText = call('copyText');
export const analyzeText = call('analyzeText');
export const getAppInfo = call('getAppInfo');
export const openLogsFolder = call('openLogsFolder');
export const openDataFolder = call('openDataFolder');

/** Subscriptions return synchronously; the underlying listener attaches once the backend resolves. */
function subscribeLater(key: 'onIndexProgress' | 'onIndexFinished') {
  return (cb: (p: IndexProgress) => void): (() => void) => {
    let unsub: (() => void) | null = null;
    let cancelled = false;
    resolveBackend().then((b) => {
      if (cancelled) return;
      unsub = b[key](cb);
    });
    return () => { cancelled = true; unsub?.(); };
  };
}
export const onIndexProgress = subscribeLater('onIndexProgress');
export const onIndexFinished = subscribeLater('onIndexFinished');

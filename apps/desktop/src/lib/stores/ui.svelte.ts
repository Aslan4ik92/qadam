import type { Theme } from '../api/types';

const LS_KEY = 'qidir.ui.v1';

interface PersistedUi {
  sidebarCollapsed: boolean;
  previewCollapsed: boolean;
  previewWidth: number;
  previewMono: boolean;
  sort: string;
}

function readPersisted(): Partial<PersistedUi> {
  try {
    const raw = localStorage.getItem(LS_KEY);
    return raw ? (JSON.parse(raw) as Partial<PersistedUi>) : {};
  } catch { return {}; }
}

export interface Toast {
  id: number;
  kind: 'info' | 'success' | 'error';
  title: string;
  text?: string;
  /** Optional actions, e.g. "Start indexing" / "Later". */
  actions?: Array<{ label: string; run: () => void; primary?: boolean }>;
  timeout: number;
}

export interface ContextMenuState {
  x: number;
  y: number;
  items: Array<{ id: string; label: string; icon?: string; shortcut?: string; run: () => void; separatorBefore?: boolean }>;
}

export interface ConfirmState {
  title: string;
  text: string;
  okLabel?: string;
  danger?: boolean;
  resolve: (ok: boolean) => void;
}

class UiState {
  #persisted = readPersisted();

  sidebarCollapsed = $state<boolean>(this.#persisted.sidebarCollapsed ?? false);
  previewCollapsed = $state<boolean>(this.#persisted.previewCollapsed ?? false);
  previewWidth = $state<number>(this.#persisted.previewWidth ?? 520);
  previewMono = $state<boolean>(this.#persisted.previewMono ?? true);
  /** Sort order persisted across sessions; the search store reads it on startup. */
  persistedSort = this.#persisted.sort ?? 'relevance';

  settingsOpen = $state(false);
  settingsTab = $state<'locations' | 'indexing' | 'fileTypes' | 'appearance' | 'about'>('locations');
  helpOpen = $state(false);
  toasts = $state<Toast[]>([]);
  contextMenu = $state<ContextMenuState | null>(null);
  confirm = $state<ConfirmState | null>(null);
  /** Effective theme ('light' | 'dark') after resolving 'system'. */
  resolvedTheme = $state<'light' | 'dark'>('light');

  #toastSeq = 0;
  #media: MediaQueryList | null = null;
  #themeSetting: Theme = 'system';

  persist(sort?: string): void {
    try {
      const data: PersistedUi = {
        sidebarCollapsed: this.sidebarCollapsed,
        previewCollapsed: this.previewCollapsed,
        previewWidth: this.previewWidth,
        previewMono: this.previewMono,
        sort: sort ?? this.persistedSort
      };
      if (sort) this.persistedSort = sort;
      localStorage.setItem(LS_KEY, JSON.stringify(data));
    } catch { /* storage unavailable */ }
  }

  toggleSidebar(): void { this.sidebarCollapsed = !this.sidebarCollapsed; this.persist(); }
  togglePreview(): void { this.previewCollapsed = !this.previewCollapsed; this.persist(); }
  setPreviewWidth(w: number): void { this.previewWidth = Math.round(w); this.persist(); }
  setPreviewMono(v: boolean): void { this.previewMono = v; this.persist(); }

  toast(kind: Toast['kind'], title: string, text?: string, opts?: { actions?: Toast['actions']; timeout?: number }): number {
    const id = ++this.#toastSeq;
    const timeout = opts?.timeout ?? (kind === 'error' ? 7000 : opts?.actions ? 12000 : 3200);
    this.toasts = [...this.toasts, { id, kind, title, text, actions: opts?.actions, timeout }];
    if (this.toasts.length > 5) this.toasts = this.toasts.slice(-5);
    return id;
  }
  error(title: string, text?: string): number { return this.toast('error', title, text); }
  success(title: string, text?: string): number { return this.toast('success', title, text); }
  info(title: string, text?: string): number { return this.toast('info', title, text); }
  dismissToast(id: number): void { this.toasts = this.toasts.filter((x) => x.id !== id); }

  openContextMenu(x: number, y: number, items: ContextMenuState['items']): void { this.contextMenu = { x, y, items }; }
  closeContextMenu(): void { this.contextMenu = null; }

  ask(title: string, text: string, okLabel?: string, danger = false): Promise<boolean> {
    return new Promise((resolve) => {
      this.confirm = { title, text, okLabel, danger, resolve: (ok) => { this.confirm = null; resolve(ok); } };
    });
  }

  openSettings(tab?: UiState['settingsTab']): void {
    if (tab) this.settingsTab = tab;
    this.settingsOpen = true;
  }

  /** Applies the theme to <html data-theme> and tracks the OS preference while 'system' is selected. */
  applyTheme(theme: Theme): void {
    this.#themeSetting = theme;
    if (typeof window === 'undefined') return;
    if (!this.#media) {
      this.#media = window.matchMedia('(prefers-color-scheme: dark)');
      this.#media.addEventListener('change', () => { if (this.#themeSetting === 'system') this.applyTheme('system'); });
    }
    const resolved: 'light' | 'dark' = theme === 'system' ? (this.#media.matches ? 'dark' : 'light') : theme;
    this.resolvedTheme = resolved;
    document.documentElement.setAttribute('data-theme', resolved);
    document.documentElement.style.colorScheme = resolved;
  }
}

export const ui = new UiState();

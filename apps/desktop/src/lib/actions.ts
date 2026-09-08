import * as api from './api/backend';
import { errorMessage } from './api/backend';
import type { SearchHit } from './api/types';
import { t } from './i18n';
import { ui } from './stores/ui.svelte';
import { indexing } from './stores/indexing.svelte';
import { settings } from './stores/settings.svelte';

async function guard(p: Promise<unknown>): Promise<boolean> {
  try { await p; return true; } catch (e) { ui.error(t('toast.error'), errorMessage(e)); return false; }
}

export const actions = {
  open(hit: SearchHit | null) { if (hit) void guard(api.openFile(hit.path)); },
  openWith(hit: SearchHit | null) { if (hit) void guard(api.openWith(hit.path)); },
  reveal(hit: SearchHit | null) { if (hit) void guard(api.revealInExplorer(hit.path)); },
  async copyPath(hit: SearchHit | null) { if (hit && await guard(api.copyText(hit.path))) ui.success(t('toast.copiedPath')); },
  async copyName(hit: SearchHit | null) { if (hit && await guard(api.copyText(hit.name))) ui.success(t('toast.copiedName')); },
  async copyText(text: string, toast?: string) { if (await guard(api.copyText(text)) && toast) ui.success(toast); },

  contextMenu(e: MouseEvent, hit: SearchHit) {
    e.preventDefault();
    ui.openContextMenu(e.clientX, e.clientY, [
      { id: 'open', label: t('results.open'), icon: 'open', shortcut: 'Enter', run: () => actions.open(hit) },
      { id: 'openWith', label: t('results.openWith'), icon: 'apps', run: () => actions.openWith(hit) },
      { id: 'reveal', label: t('results.reveal'), icon: 'explorer', shortcut: 'Ctrl+Enter', run: () => actions.reveal(hit) },
      { id: 'copyPath', label: t('results.copyPath'), icon: 'copy', shortcut: 'Ctrl+C', run: () => void actions.copyPath(hit), separatorBefore: true },
      { id: 'copyName', label: t('results.copyName'), icon: 'copy', run: () => void actions.copyName(hit) }
    ]);
  },

  /** Native folder picker → save roots → offer to index. Returns the number of roots added. */
  async addFolders(): Promise<number> {
    let picked: string[] = [];
    try { picked = await api.pickFolders(); } catch (e) { ui.error(t('toast.error'), errorMessage(e)); return 0; }
    if (picked.length === 0) return 0;
    const added = await settings.addRoots(picked);
    if (added > 0) actions.offerIndexing(added);
    return added;
  },

  offerIndexing(added: number) {
    ui.toast('success', t('onboarding.added', { count: added }), t('onboarding.askIndex'), {
      actions: [
        { label: t('onboarding.startIndexing'), primary: true, run: () => void indexing.start(false) },
        { label: t('common.later'), run: () => {} }
      ]
    });
  }
};

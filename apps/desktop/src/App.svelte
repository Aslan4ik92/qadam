<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from './lib/i18n';
  import { settings } from './lib/stores/settings.svelte';
  import { search } from './lib/stores/search.svelte';
  import { indexing } from './lib/stores/indexing.svelte';
  import { ui } from './lib/stores/ui.svelte';
  import { actions } from './lib/actions';
  import { matchesChord, isEditableTarget } from './lib/utils/keys';
  import TopBar from './lib/components/TopBar.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import ResultsList from './lib/components/ResultsList.svelte';
  import PreviewPane from './lib/components/PreviewPane.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import SettingsDialog from './lib/components/SettingsDialog.svelte';
  import ContextMenu from './lib/components/ContextMenu.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import Splitter from './lib/components/Splitter.svelte';
  import Icon from './lib/components/Icon.svelte';

  let searchInput = $state<HTMLInputElement | null>(null);
  let mainEl = $state<HTMLElement | null>(null);

  onMount(() => {
    void settings.load();
    indexing.init();
    const off = indexing.onFinished(() => { if (search.active) void search.run(); });
    searchInput?.focus();
    return () => { off(); indexing.destroy(); };
  });

  // Clamp the preview width to the available space.
  function clampPreview(w: number): number {
    const total = mainEl?.clientWidth ?? 1400;
    return Math.min(Math.max(w, 280), Math.max(280, total - (ui.sidebarCollapsed ? 0 : 260) - 360));
  }

  function onKeydown(e: KeyboardEvent) {
    if (ui.settingsOpen || ui.confirm) return; // the dialog handles its own keys
    const editable = isEditableTarget(e);
    const inSearch = e.target === searchInput;

    if (matchesChord(e, 'Ctrl+F') || matchesChord(e, 'Ctrl+K')) { e.preventDefault(); searchInput?.focus(); searchInput?.select(); return; }
    if (matchesChord(e, 'Ctrl+,')) { e.preventDefault(); ui.openSettings(); return; }
    if (matchesChord(e, 'Ctrl+B')) { e.preventDefault(); ui.toggleSidebar(); return; }
    if (matchesChord(e, 'Ctrl+P')) { e.preventDefault(); ui.togglePreview(); return; }
    if (matchesChord(e, 'F5')) { e.preventDefault(); if (!indexing.running) void indexing.start(false); return; }
    if (matchesChord(e, 'F3')) { e.preventDefault(); search.nextMatch(1); return; }
    if (matchesChord(e, 'Shift+F3')) { e.preventDefault(); search.nextMatch(-1); return; }
    if (e.key === 'Escape') {
      if (ui.contextMenu) { ui.closeContextMenu(); return; }
      if (ui.helpOpen) { ui.helpOpen = false; return; }
      if (!inSearch) { searchInput?.focus(); return; }
      return; // TopBar clears the query itself
    }
    if (editable && !inSearch) return;

    if (e.key === 'ArrowDown') { e.preventDefault(); search.moveSelection(1); return; }
    if (e.key === 'ArrowUp') { e.preventDefault(); search.moveSelection(-1); return; }
    if (e.key === 'PageDown') { e.preventDefault(); search.moveSelection(10); return; }
    if (e.key === 'PageUp') { e.preventDefault(); search.moveSelection(-10); return; }
    if (e.key === 'Home' && !inSearch) { e.preventDefault(); search.select(0); return; }
    if (e.key === 'End' && !inSearch) { e.preventDefault(); search.select(search.hits.length - 1); return; }
    if (matchesChord(e, 'Ctrl+Enter')) { e.preventDefault(); actions.reveal(search.selected); return; }
    if (e.key === 'Enter' && !inSearch) { e.preventDefault(); actions.open(search.selected); return; }
    if (matchesChord(e, 'Ctrl+C')) {
      if (inSearch && searchInput && searchInput.selectionStart !== searchInput.selectionEnd) return; // copying text in the input
      if (!inSearch && window.getSelection()?.toString()) return; // copying selected preview text
      if (search.selected) { e.preventDefault(); void actions.copyPath(search.selected); }
      return;
    }
    if (matchesChord(e, 'Ctrl+Shift+C')) { e.preventDefault(); void actions.copyName(search.selected); return; }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <TopBar bind:inputRef={searchInput} />
  <main class="main" bind:this={mainEl}>
    {#if !ui.sidebarCollapsed}
      <div class="col sidebar">
        <Sidebar />
      </div>
    {/if}
    <button class="edge-btn left" onclick={() => ui.toggleSidebar()} title="{ui.sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')} (Ctrl+B)" aria-label={ui.sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')}>
      <Icon name={ui.sidebarCollapsed ? 'chevronRight' : 'chevronLeft'} size={12} />
    </button>
    <div class="col center">
      <ResultsList />
    </div>
    {#if !ui.previewCollapsed}
      <Splitter label={t('preview.title')} onresize={(dx) => (ui.previewWidth = clampPreview(ui.previewWidth - dx))} onend={() => ui.setPreviewWidth(clampPreview(ui.previewWidth))} />
      <div class="col preview" style:width="{ui.previewWidth}px">
        <PreviewPane />
      </div>
    {:else}
      <button class="edge-btn right" onclick={() => ui.togglePreview()} title="{t('preview.expand')} (Ctrl+P)" aria-label={t('preview.expand')}>
        <Icon name="chevronLeft" size={12} />
      </button>
    {/if}
  </main>
  <StatusBar />
</div>

{#if ui.settingsOpen}<SettingsDialog />{/if}
<ContextMenu />
<Toasts />

{#if ui.confirm}
  {@const c = ui.confirm}
  <div class="backdrop" role="presentation">
    <div class="confirm card" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
      <h3 id="confirm-title">{c.title}</h3>
      <p>{c.text}</p>
      <div class="cbtns">
        <button class="btn" onclick={() => c.resolve(false)}>{t('common.cancel')}</button>
        <button class="btn primary" class:danger-btn={c.danger} onclick={() => c.resolve(true)}>{c.okLabel ?? t('common.ok')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .app { height: 100%; display: flex; flex-direction: column; min-width: 960px; }
  .main { flex: 1 1 auto; min-height: 0; display: flex; padding: 0 8px 8px; gap: 0; position: relative; }
  .col { min-height: 0; }
  .sidebar { width: 260px; flex: 0 0 260px; }
  .center { flex: 1 1 auto; min-width: 320px; }
  .preview { flex: 0 0 auto; min-width: 280px; }
  .edge-btn {
    position: absolute; top: 50%; transform: translateY(-50%); z-index: 3; width: 14px; height: 48px;
    display: grid; place-items: center; color: var(--fg-3); background: var(--surface); border: 1px solid var(--stroke); border-radius: 4px;
    opacity: 0; transition: opacity var(--dur) var(--ease), color var(--dur) var(--ease);
  }
  .edge-btn.left { left: 0; }
  .edge-btn.right { right: 2px; }
  .main:hover .edge-btn, .edge-btn:focus-visible { opacity: 1; }
  .edge-btn:hover { color: var(--accent); }
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.35); display: flex; align-items: center; justify-content: center; z-index: 120; animation: fade-in var(--dur) var(--ease); }
  .confirm { width: 420px; max-width: calc(100vw - 48px); padding: 20px 22px; box-shadow: var(--shadow-dialog); display: flex; flex-direction: column; gap: 10px; }
  .confirm h3 { font-size: var(--fs-xl); font-weight: 600; }
  .confirm p { color: var(--fg-2); line-height: 1.5; }
  .cbtns { display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px; }
  .danger-btn { background: var(--danger); color: #fff; }
</style>

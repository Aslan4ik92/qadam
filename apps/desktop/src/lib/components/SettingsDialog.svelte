<script lang="ts">
  import { t, LANGUAGE_NAMES } from '../i18n';
  import { settings } from '../stores/settings.svelte';
  import { indexing } from '../stores/indexing.svelte';
  import { ui } from '../stores/ui.svelte';
  import * as api from '../api/backend';
  import { errorMessage } from '../api/backend';
  import { DEFAULT_EXCLUDE_GLOBS } from '../utils/query';
  import { formatNumber, formatSize } from '../utils/format';
  import Icon from './Icon.svelte';
  import Toggle from './Toggle.svelte';
  import type { AppInfo, DriveInfo, Settings, Theme, UiLanguage } from '../api/types';

  type Tab = typeof ui.settingsTab;
  const TABS: Array<{ id: Tab; icon: string }> = [
    { id: 'locations', icon: 'folder' }, { id: 'indexing', icon: 'refresh' }, { id: 'fileTypes', icon: 'fileDoc' },
    { id: 'appearance', icon: 'eye' }, { id: 'about', icon: 'info' }
  ];
  const tabLabel = (id: Tab) => t(`settings.tab.${id}`);

  // Draft copy: edits are applied on Save.
  let draft = $state<Settings>(structuredClone($state.snapshot(settings.value)));
  let excludeText = $state(draft.excludeGlobs.join('\n'));
  let includeInput = $state('');
  let excludeInput = $state('');
  let drives = $state<DriveInfo[]>([]);
  let drivesOpen = $state(false);
  let info = $state<AppInfo | null>(null);
  let dialog = $state<HTMLElement | null>(null);
  const lang = $derived(settings.value.uiLanguage);
  const dirty = $derived(JSON.stringify({ ...draft, excludeGlobs: parseGlobs(excludeText) }) !== JSON.stringify(settings.value));

  function parseGlobs(text: string): string[] { return text.split('\n').map((s) => s.trim()).filter(Boolean); }

  $effect(() => {
    api.getAppInfo().then((i) => (info = i)).catch(() => {});
    api.listDrives().then((d) => (drives = d)).catch(() => (drives = []));
    // Focus the dialog and trap Escape.
    dialog?.querySelector<HTMLElement>('button')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.stopPropagation(); if (drivesOpen) drivesOpen = false; else close(); }
      if (e.key === 'Tab' && dialog) {
        const f = [...dialog.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), select, textarea, [tabindex="0"]')];
        if (f.length === 0) return;
        const first = f[0], last = f[f.length - 1];
        if (e.shiftKey && document.activeElement === first) { last.focus(); e.preventDefault(); }
        else if (!e.shiftKey && document.activeElement === last) { first.focus(); e.preventDefault(); }
      }
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  });

  function close() { ui.settingsOpen = false; }

  async function save() {
    const next: Settings = { ...$state.snapshot(draft), excludeGlobs: parseGlobs(excludeText) };
    const ok = await settings.save(next);
    if (ok) { ui.success(t('settings.saved')); close(); }
  }

  async function addFolder() {
    try {
      const picked = await api.pickFolders();
      addRoots(picked);
    } catch (e) { ui.error(t('toast.error'), errorMessage(e)); }
  }
  function addRoots(paths: string[]) {
    const have = new Set(draft.roots.map((r) => r.path.toLowerCase()));
    for (const p of paths) if (p && !have.has(p.toLowerCase())) { draft.roots.push({ path: p, enabled: true }); have.add(p.toLowerCase()); }
  }
  function removeRoot(i: number) { draft.roots.splice(i, 1); }

  function addExt(list: 'includeExtensions' | 'excludeExtensions', raw: string) {
    const parts = raw.split(/[\s,;]+/).map((s) => s.trim().replace(/^\./, '').toLowerCase()).filter(Boolean);
    for (const p of parts) if (!draft[list].includes(p)) draft[list].push(p);
    if (list === 'includeExtensions') includeInput = ''; else excludeInput = '';
  }
  function extKey(e: KeyboardEvent, list: 'includeExtensions' | 'excludeExtensions') {
    const input = e.currentTarget as HTMLInputElement;
    if (e.key === 'Enter' || e.key === ',' || e.key === ' ') { e.preventDefault(); addExt(list, input.value); }
    else if (e.key === 'Backspace' && input.value === '' && draft[list].length) draft[list].pop();
  }

  async function openFolder(kind: 'data' | 'logs') {
    try { await (kind === 'data' ? api.openDataFolder() : api.openLogsFolder()); } catch (e) { ui.error(t('toast.error'), errorMessage(e)); }
  }
  async function runIndex(full: boolean) { if (await indexing.start(full)) close(); }
</script>

<div class="backdrop" role="presentation" onmousedown={(e) => { if (e.target === e.currentTarget) close(); }}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" bind:this={dialog}>
    <nav class="tabs" aria-label={t('settings.title')}>
      <h2 id="settings-title">{t('settings.title')}</h2>
      {#each TABS as tab (tab.id)}
        <button class="tab" class:on={ui.settingsTab === tab.id} role="tab" aria-selected={ui.settingsTab === tab.id} onclick={() => (ui.settingsTab = tab.id)}>
          <Icon name={tab.icon} size={16} /> {tabLabel(tab.id)}
        </button>
      {/each}
    </nav>

    <div class="panel">
      <div class="panel-head">
        <h3>{tabLabel(ui.settingsTab)}</h3>
        <button class="btn icon subtle" onclick={close} aria-label={t('common.close')}><Icon name="close" size={16} /></button>
      </div>

      <div class="content" role="tabpanel">
        {#if ui.settingsTab === 'locations'}
          <div class="roots">
            {#if draft.roots.length === 0}
              <div class="faint empty-roots">{t('settings.roots.empty')}</div>
            {:else}
              <table>
                <thead><tr><th class="w-en">{t('settings.roots.enabled')}</th><th>{t('settings.roots.path')}</th><th class="w-rm"></th></tr></thead>
                <tbody>
                  {#each draft.roots as root, i (root.path)}
                    <tr class:off={!root.enabled}>
                      <td><Toggle checked={root.enabled} onchange={(v) => (draft.roots[i].enabled = v)} /></td>
                      <td class="path selectable" title={root.path}><Icon name="folder" size={15} /> <span class="ellipsis">{root.path}</span></td>
                      <td><button class="btn sm icon subtle danger" onclick={() => removeRoot(i)} aria-label={t('settings.roots.remove')} title={t('settings.roots.remove')}><Icon name="trash" size={15} /></button></td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
            <div class="row-btns">
              <button class="btn" onclick={addFolder}><Icon name="folderAdd" size={15} /> {t('sidebar.addFolder')}</button>
              <div class="drive-anchor">
                <button class="btn" onclick={() => (drivesOpen = !drivesOpen)} aria-expanded={drivesOpen} disabled={drives.length === 0}><Icon name="drive" size={15} /> {t('settings.roots.addDrive')} <Icon name="chevronDown" size={12} /></button>
                {#if drivesOpen}
                  <div class="flyout drives" role="menu">
                    {#each drives as d (d.path)}
                      <button class="menu-item" role="menuitem" disabled={draft.roots.some((r) => r.path.toLowerCase() === d.path.toLowerCase())} onclick={() => { addRoots([d.path]); drivesOpen = false; }}>
                        <Icon name="drive" size={15} /> {d.label} <span class="shortcut">{d.path}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
          <div class="field">
            <div class="field-head">
              <label for="excl">{t('settings.exclusions')}</label>
              <button class="link" onclick={() => (excludeText = DEFAULT_EXCLUDE_GLOBS.join('\n'))}>{t('settings.restoreDefaults')}</button>
            </div>
            <textarea id="excl" class="input mono" rows="7" spellcheck="false" bind:value={excludeText}></textarea>
            <div class="hint">{t('settings.exclusions.hint')}</div>
          </div>

        {:else if ui.settingsTab === 'indexing'}
          <div class="field inline">
            <label for="maxsize">{t('settings.maxFileSize')}</label>
            <input id="maxsize" class="input num" type="number" min="1" max="4096" value={Math.round(draft.maxFileSizeBytes / (1024 * 1024))} onchange={(e) => (draft.maxFileSizeBytes = Math.max(1, Number((e.currentTarget as HTMLInputElement).value) || 1) * 1024 * 1024)} />
          </div>
          <div class="hint">{t('settings.maxFileSize.hint')}</div>
          <Toggle label={t('settings.storeContent')} hint={t('settings.storeContent.hint')} checked={draft.storeContent} onchange={(v) => (draft.storeContent = v)} />
          <Toggle label={t('settings.extractPdf')} checked={draft.extractPdf} onchange={(v) => (draft.extractPdf = v)} />
          <Toggle label={t('settings.indexHidden')} checked={draft.indexHidden} onchange={(v) => (draft.indexHidden = v)} />
          <Toggle label={t('settings.followLinks')} checked={draft.followLinks} onchange={(v) => (draft.followLinks = v)} />
          <Toggle label={t('settings.watchChanges')} checked={draft.watchChanges} onchange={(v) => (draft.watchChanges = v)} />
          <Toggle label={t('settings.reindexOnStart')} checked={draft.reindexOnStart} onchange={(v) => (draft.reindexOnStart = v)} />
          <div class="grid2">
            <div class="field inline">
              <label for="threads">{t('settings.workerThreads')} <span class="faint">({t('settings.workerThreads.hint')})</span></label>
              <input id="threads" class="input num" type="number" min="0" max="64" bind:value={draft.workerThreads} />
            </div>
            <div class="field inline">
              <label for="mem">{t('settings.writerMemory')}</label>
              <input id="mem" class="input num" type="number" min="64" max="8192" step="64" bind:value={draft.writerMemoryMb} />
            </div>
          </div>
          <div class="row-btns top-gap">
            <button class="btn" onclick={() => runIndex(false)} disabled={indexing.running}><Icon name="refresh" size={15} /> {t('index.update')}</button>
            <button class="btn" onclick={() => runIndex(true)} disabled={indexing.running}><Icon name="refresh" size={15} /> {t('index.full')}</button>
            <button class="btn danger" onclick={() => indexing.clear()} disabled={indexing.running}><Icon name="trash" size={15} /> {t('index.clear')}</button>
          </div>

        {:else if ui.settingsTab === 'fileTypes'}
          <div class="field">
            <label for="inc">{t('settings.includeExt')} <span class="faint">— {t('settings.includeExt.hint')}</span></label>
            <div class="chips-input" role="group">
              {#each draft.includeExtensions as ext, i (ext)}
                <span class="chip on">.{ext}<button class="x" onclick={() => draft.includeExtensions.splice(i, 1)} aria-label={t('common.remove')}><Icon name="close" size={11} /></button></span>
              {/each}
              <input id="inc" placeholder={t('settings.ext.add')} bind:value={includeInput} onkeydown={(e) => extKey(e, 'includeExtensions')} onblur={() => includeInput && addExt('includeExtensions', includeInput)} />
            </div>
          </div>
          <div class="field">
            <label for="exc">{t('settings.excludeExt')}</label>
            <div class="chips-input" role="group">
              {#each draft.excludeExtensions as ext, i (ext)}
                <span class="chip">.{ext}<button class="x" onclick={() => draft.excludeExtensions.splice(i, 1)} aria-label={t('common.remove')}><Icon name="close" size={11} /></button></span>
              {/each}
              <input id="exc" placeholder={t('settings.ext.add')} bind:value={excludeInput} onkeydown={(e) => extKey(e, 'excludeExtensions')} onblur={() => excludeInput && addExt('excludeExtensions', excludeInput)} />
            </div>
          </div>

        {:else if ui.settingsTab === 'appearance'}
          <div class="field">
            <span class="label">{t('settings.language')}</span>
            <div class="seg" role="radiogroup">
              {#each (['ru', 'kk', 'en'] as UiLanguage[]) as l (l)}
                <button class="seg-btn" class:on={draft.uiLanguage === l} role="radio" aria-checked={draft.uiLanguage === l} onclick={() => (draft.uiLanguage = l)}>{LANGUAGE_NAMES[l]}</button>
              {/each}
            </div>
          </div>
          <div class="field">
            <span class="label">{t('settings.theme')}</span>
            <div class="seg" role="radiogroup">
              {#each (['system', 'light', 'dark'] as Theme[]) as th (th)}
                <button class="seg-btn" class:on={draft.theme === th} role="radio" aria-checked={draft.theme === th} onclick={() => (draft.theme = th)}>{t(`settings.theme.${th}`)}</button>
              {/each}
            </div>
          </div>
          <div class="field inline">
            <label for="ps">{t('settings.pageSize')}</label>
            <select id="ps" class="input" bind:value={draft.pageSize}>
              {#each [25, 50, 100, 200] as n (n)}<option value={n}>{n}</option>{/each}
            </select>
          </div>
          <Toggle label={t('settings.previewMono')} checked={ui.previewMono} onchange={(v) => ui.setPreviewMono(v)} />

        {:else}
          <div class="about">
            <div class="about-head">
              <Icon name="logo" size={48} />
              <div>
                <div class="app">QIDIR</div>
                <div class="faint">{t('app.tagline')}</div>
              </div>
            </div>
            <dl>
              <dt>{t('settings.about.version')}</dt><dd class="selectable">{info?.version ?? '—'}</dd>
              <dt>{t('settings.about.platform')}</dt><dd class="selectable">{info?.platform ?? '—'}</dd>
              {#if indexing.stats}
                <dt>{t('settings.about.documents')}</dt><dd class="tabular">{formatNumber(indexing.stats.documents, lang)}</dd>
                <dt>{t('settings.about.indexSize')}</dt><dd class="tabular">{formatSize(indexing.stats.indexSizeBytes, lang)}</dd>
              {/if}
              <dt>{t('settings.about.dataDir')}</dt><dd><span class="selectable mono-sm">{info?.dataDir ?? indexing.stats?.dataDir ?? '—'}</span> <button class="link" onclick={() => openFolder('data')}>{t('settings.about.openData')}</button></dd>
              <dt>{t('settings.about.logs')}</dt><dd><span class="selectable mono-sm">{info?.logDir ?? '—'}</span> <button class="link" onclick={() => openFolder('logs')}>{t('settings.about.openLogs')}</button></dd>
            </dl>
            <p class="credits">{t('settings.about.credits')}</p>
            <p class="faint">{t('settings.about.license')} · © 2026</p>
          </div>
        {/if}
      </div>

      <div class="foot">
        <span class="faint">{#if settings.saving}{t('common.loading')}{/if}</span>
        <button class="btn" onclick={close}>{t('common.cancel')}</button>
        <button class="btn primary" onclick={save} disabled={!dirty || settings.saving}>{t('settings.save')}</button>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.35); display: flex; align-items: center; justify-content: center; z-index: 100; animation: fade-in var(--dur) var(--ease); }
  .dialog { width: 880px; max-width: calc(100vw - 48px); height: min(740px, calc(100vh - 48px)); display: flex; background: var(--surface); border: 1px solid var(--stroke); border-radius: var(--radius); box-shadow: var(--shadow-dialog); overflow: hidden; }
  .tabs { width: 212px; flex: 0 0 auto; padding: 16px 8px; background: var(--surface-2); border-right: 1px solid var(--divider); display: flex; flex-direction: column; gap: 2px; }
  .tabs h2 { font-size: var(--fs-xl); font-weight: 600; padding: 4px 12px 14px; }
  .tab { display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 12px; border-radius: var(--radius-sm); text-align: left; color: var(--fg-2); position: relative; font-size: var(--fs-md); }
  .tab:hover { background: var(--surface-hover); color: var(--fg); }
  .tab.on { background: var(--surface-active); color: var(--fg); font-weight: 600; }
  .tab.on::before { content: ''; position: absolute; left: 0; top: 10px; bottom: 10px; width: 3px; border-radius: 2px; background: var(--accent); }
  .panel { flex: 1 1 auto; min-width: 0; display: flex; flex-direction: column; }
  .panel-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 16px 8px 24px; }
  .panel-head h3 { font-size: var(--fs-xl); font-weight: 600; }
  .content { flex: 1 1 auto; min-height: 0; overflow: auto; padding: 4px 24px 16px; display: flex; flex-direction: column; gap: 12px; }
  .foot { display: flex; align-items: center; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--divider); background: var(--surface-2); }
  .foot span { margin-right: auto; }

  table { width: 100%; border-collapse: collapse; font-size: var(--fs-md); }
  th { text-align: left; font-size: var(--fs-xs); color: var(--fg-3); font-weight: 600; padding: 0 8px 6px; text-transform: uppercase; letter-spacing: 0.04em; }
  td { padding: 2px 8px; border-top: 1px solid var(--divider); vertical-align: middle; }
  .w-en { width: 60px; } .w-rm { width: 40px; }
  tr.off .path { color: var(--fg-3); }
  .path { display: flex; align-items: center; gap: 8px; min-width: 0; max-width: 460px; }
  .path :global(svg) { flex: 0 0 auto; color: var(--fg-3); }
  .empty-roots { padding: 14px 0; }
  .row-btns { display: flex; gap: 8px; margin-top: 10px; flex-wrap: wrap; }
  .top-gap { margin-top: 18px; padding-top: 14px; border-top: 1px solid var(--divider); }
  .drive-anchor { position: relative; }
  .drives { position: absolute; top: calc(100% + 4px); left: 0; z-index: 10; min-width: 240px; }
  .drives .menu-item:disabled { color: var(--fg-disabled); }

  .field { display: flex; flex-direction: column; gap: 6px; }
  .field.inline { flex-direction: row; align-items: center; justify-content: space-between; gap: 16px; padding: 4px 0; }
  .field-head { display: flex; justify-content: space-between; align-items: center; }
  .field label, .field .label { font-size: var(--fs-md); }
  .num { width: 110px; text-align: right; }
  .hint { font-size: var(--fs-sm); color: var(--fg-2); margin-top: -6px; }
  .mono { font-family: var(--font-mono); font-size: var(--fs-sm); }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 0 24px; }
  .link { color: var(--accent); font-size: var(--fs-sm); }
  .link:hover { text-decoration: underline; }

  .chips-input { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; min-height: 40px; padding: 6px 8px; border-radius: var(--radius-sm); background: var(--surface); border: 1px solid var(--stroke-control); border-bottom-color: var(--stroke-control-strong); }
  .chips-input:focus-within { border-bottom: 2px solid var(--accent); }
  .chips-input input { flex: 1 1 160px; min-width: 120px; border: 0; background: transparent; outline: none; height: 24px; user-select: text; }
  .chips-input .x { display: inline-flex; margin-left: 2px; padding: 2px; border-radius: 50%; color: inherit; opacity: 0.7; }
  .chips-input .x:hover { opacity: 1; background: rgba(0,0,0,0.08); }

  .seg { display: inline-flex; background: var(--surface-3); border-radius: 6px; padding: 3px; gap: 2px; align-self: flex-start; }
  .seg-btn { height: 30px; padding: 0 16px; border-radius: 4px; font-size: var(--fs-md); color: var(--fg-2); transition: background var(--dur) var(--ease), color var(--dur) var(--ease); }
  .seg-btn:hover { color: var(--fg); }
  .seg-btn.on { background: var(--surface); color: var(--accent); font-weight: 600; box-shadow: var(--shadow-card); }

  .about { display: flex; flex-direction: column; gap: 16px; }
  .about-head { display: flex; align-items: center; gap: 14px; }
  .app { font-size: 20px; font-weight: 700; letter-spacing: 0.06em; }
  dl { display: grid; grid-template-columns: 180px 1fr; gap: 8px 16px; margin: 0; font-size: var(--fs-md); }
  dt { color: var(--fg-2); } dd { margin: 0; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; min-width: 0; }
  .mono-sm { font-family: var(--font-mono); font-size: var(--fs-sm); word-break: break-all; }
  .credits { color: var(--fg-2); line-height: 1.5; max-width: 560px; }
</style>

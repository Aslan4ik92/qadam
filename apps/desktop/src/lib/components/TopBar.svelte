<script lang="ts">
  import { t, tFound } from '../i18n';
  import { search, SORT_ORDERS } from '../stores/search.svelte';
  import { indexing } from '../stores/indexing.svelte';
  import { ui } from '../stores/ui.svelte';
  import Icon from './Icon.svelte';
  import Dropdown from './Dropdown.svelte';
  import HelpPopover from './HelpPopover.svelte';
  import type { SortOrder } from '../api/types';

  interface Props { inputRef?: HTMLInputElement | null }
  let { inputRef = $bindable(null) }: Props = $props();

  let sortOpen = $state(false);
  let indexOpen = $state(false);

  const sortKey = (s: SortOrder) => `sort.${s}` as const;

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); search.runNow(); }
    else if (e.key === 'Escape') {
      if (search.query) { e.preventDefault(); e.stopPropagation(); search.clear(); }
    }
  }
</script>

<header class="topbar">
  <div class="row">
    <div class="brand" title={t('app.tagline')}>
      <Icon name="logo" size={28} />
      <span class="wordmark">QIDIR</span>
    </div>

    <div class="searchbox" class:loading={search.loading}>
      <span class="sicon"><Icon name="search" size={18} /></span>
      <input
        bind:this={inputRef}
        class="sinput"
        type="search"
        spellcheck="false"
        autocomplete="off"
        placeholder={t('search.placeholder')}
        aria-label={t('search.placeholder')}
        value={search.query}
        oninput={(e) => search.setQuery((e.currentTarget as HTMLInputElement).value)}
        onkeydown={onKey}
      />
      {#if search.query}
        <button class="btn xs icon subtle clear" onclick={() => { search.clear(); inputRef?.focus(); }} aria-label={t('search.clear')} title={t('search.clear')}>
          <Icon name="close" size={14} />
        </button>
      {/if}
      <div class="modes" role="radiogroup" aria-label="Mode">
        <button class="mode" class:on={search.mode === 'smart'} role="radio" aria-checked={search.mode === 'smart'} title={t('search.mode.smart.hint')} onclick={() => search.setMode('smart')}>{t('search.mode.smart')}</button>
        <button class="mode" class:on={search.mode === 'exact'} role="radio" aria-checked={search.mode === 'exact'} title={t('search.mode.exact.hint')} onclick={() => search.setMode('exact')}>{t('search.mode.exact')}</button>
      </div>
    </div>

    <Dropdown bind:open={sortOpen} align="right" width={200}>
      {#snippet trigger({ toggle })}
        <button class="btn" onclick={toggle} aria-haspopup="menu" aria-expanded={sortOpen} title={t('sort.label')}>
          <Icon name="sort" size={16} />
          <span class="sort-label">{t(sortKey(search.sort))}</span>
          <Icon name="chevronDown" size={12} />
        </button>
      {/snippet}
      {#snippet children({ close })}
        {#each SORT_ORDERS as s (s)}
          <button class="menu-item" class:checked={s === search.sort} role="menuitemradio" aria-checked={s === search.sort} onclick={() => { search.setSort(s); close(); }}>
            <span class="mi">{#if s === search.sort}<Icon name="check" size={14} />{/if}</span>{t(sortKey(s))}
          </button>
        {/each}
      {/snippet}
    </Dropdown>

    <div class="split">
      {#if indexing.running}
        <button class="btn" onclick={() => indexing.cancel()} title={t('index.cancel')}>
          <Icon name="stop" size={14} /> {t('index.cancel')}
        </button>
      {:else}
        <button class="btn main" onclick={() => indexing.start(false)} title={t('index.update')} disabled={indexing.running}>
          <Icon name="refresh" size={15} /> {t('index.reindex')}
        </button>
      {/if}
      <Dropdown bind:open={indexOpen} align="right" width={230}>
        {#snippet trigger({ toggle })}
          <button class="btn icon arrow" onclick={toggle} aria-haspopup="menu" aria-expanded={indexOpen} aria-label={t('common.more')}>
            <Icon name="chevronDown" size={12} />
          </button>
        {/snippet}
        {#snippet children({ close })}
          <button class="menu-item" role="menuitem" disabled={indexing.running} onclick={() => { close(); void indexing.start(false); }}><span class="mi"><Icon name="refresh" size={15} /></span>{t('index.update')}</button>
          <button class="menu-item" role="menuitem" disabled={indexing.running} onclick={() => { close(); void indexing.start(true); }}><span class="mi"><Icon name="refresh" size={15} /></span>{t('index.full')}</button>
          {#if indexing.running}
            <div class="menu-sep"></div>
            <button class="menu-item" role="menuitem" onclick={() => { close(); void indexing.cancel(); }}><span class="mi"><Icon name="stop" size={14} /></span>{t('index.cancel')}</button>
          {/if}
        {/snippet}
      </Dropdown>
    </div>

    <button class="btn icon" onclick={() => ui.openSettings()} title="{t('common.settings')} (Ctrl+,)" aria-label={t('common.settings')}>
      <Icon name="settings" size={17} />
    </button>
  </div>

  <div class="hint">
    <div class="help-anchor">
      <button class="btn xs icon subtle" class:on={ui.helpOpen} onclick={() => (ui.helpOpen = !ui.helpOpen)} aria-label={t('search.help')} title={t('search.help')} aria-expanded={ui.helpOpen}>
        <Icon name="help" size={15} />
      </button>
      {#if ui.helpOpen}<HelpPopover onclose={() => (ui.helpOpen = false)} />{/if}
    </div>
    {#if search.loading && !search.searched}
      <span class="muted">{t('search.searching')}</span>
    {:else if search.searched && !search.error}
      <span class="found">{tFound(search.total, search.tookMs)}</span>
      {#if search.interpretation}
        <span class="sep">·</span>
        <span class="interp ellipsis" title={search.interpretation}><span class="faint">{t('search.interpretation')}:</span> <code>{search.interpretation}</code></span>
      {/if}
    {:else}
      <span class="faint">{t('search.hint.empty')}</span>
    {/if}
  </div>
</header>

<style>
  .topbar { padding: 10px 14px 6px; background: var(--bg); display: flex; flex-direction: column; gap: 4px; }
  .row { display: flex; align-items: center; gap: 10px; }
  .brand { display: flex; align-items: center; gap: 8px; padding-right: 6px; flex: 0 0 auto; }
  .wordmark { font-weight: 700; font-size: 17px; letter-spacing: 0.06em; color: var(--fg); }
  .searchbox {
    flex: 1 1 auto; min-width: 200px; display: flex; align-items: center; height: 38px; padding: 0 6px 0 10px;
    background: var(--surface); border: 1px solid var(--stroke-control); border-bottom: 1px solid var(--stroke-control-strong);
    border-radius: var(--radius); gap: 6px; position: relative; transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);
  }
  .searchbox:focus-within { border-bottom: 2px solid var(--accent); }
  .searchbox.loading::after {
    content: ''; position: absolute; left: 8px; right: 8px; bottom: -1px; height: 2px; border-radius: 1px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent); background-size: 40% 100%;
    animation: slide 1s linear infinite;
  }
  @keyframes slide { from { background-position: -40% 0; } to { background-position: 140% 0; } }
  .sicon { color: var(--fg-3); flex: 0 0 auto; }
  .sinput { flex: 1 1 auto; min-width: 0; height: 100%; border: 0; background: transparent; font-size: 15px; outline: none; user-select: text; }
  .sinput::-webkit-search-cancel-button { display: none; }
  .sinput::placeholder { color: var(--fg-3); }
  .clear { color: var(--fg-2); }
  .modes { display: inline-flex; background: var(--surface-3); border-radius: 6px; padding: 2px; gap: 2px; flex: 0 0 auto; }
  .mode { height: 26px; padding: 0 12px; border-radius: 4px; font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); transition: background var(--dur) var(--ease), color var(--dur) var(--ease); }
  .mode:hover { color: var(--fg); }
  .mode.on { background: var(--surface); color: var(--accent); box-shadow: var(--shadow-card); }
  .sort-label { max-width: 150px; overflow: hidden; text-overflow: ellipsis; }
  .split { display: inline-flex; }
  .split .main { border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .split .arrow { border-top-left-radius: 0; border-bottom-left-radius: 0; margin-left: -1px; width: 26px; }
  .mi { width: 16px; display: inline-flex; color: var(--fg-2); }
  .menu-item.checked .mi { color: var(--accent); }
  .menu-item:disabled { color: var(--fg-disabled); }
  .hint { display: flex; align-items: center; gap: 8px; min-height: 22px; padding-left: 122px; font-size: var(--fs-sm); color: var(--fg-2); }
  .help-anchor { position: relative; display: inline-flex; }
  .help-anchor .on { background: var(--accent-soft); color: var(--accent); }
  .found { font-variant-numeric: tabular-nums; }
  .sep { color: var(--fg-3); }
  .interp { min-width: 0; max-width: 60%; }
  .interp code { font-family: var(--font-mono); font-size: var(--fs-xs); background: var(--surface-3); padding: 1px 6px; border-radius: 4px; }
</style>

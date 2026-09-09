<script lang="ts">
  import { t } from '../i18n';
  import { search, hasActiveFilters, type ModifiedPreset, type SizePreset } from '../stores/search.svelte';
  import { settings } from '../stores/settings.svelte';
  import { actions } from '../actions';
  import { baseName } from '../utils/path';
  import { formatNumber } from '../utils/format';
  import Checkbox from './Checkbox.svelte';
  import Icon, { CATEGORY_ICON } from './Icon.svelte';
  import type { FileCategory } from '../api/types';

  const CATEGORIES: FileCategory[] = ['document', 'spreadsheet', 'presentation', 'pdf', 'text', 'code', 'ebook', 'web', 'data', 'email', 'other'];
  const MODIFIED: ModifiedPreset[] = ['any', 'today', 'week', 'month', 'year', 'custom'];
  const SIZES: SizePreset[] = ['any', 'small', 'medium', 'large', 'custom'];

  const lang = $derived(settings.value.uiLanguage);
  const f = $derived(search.filters);
  const active = $derived(hasActiveFilters(f));

  /** Locations = settings roots merged with facet counts; facets may include roots not in settings (stale index). */
  const locations = $derived.by(() => {
    const counts = new Map(search.facets.roots.map((r) => [r.value.toLowerCase(), r.count]));
    const rows = settings.value.roots.map((r) => ({ path: r.path, count: counts.get(r.path.toLowerCase()), enabled: r.enabled }));
    for (const fr of search.facets.roots) {
      if (!rows.some((r) => r.path.toLowerCase() === fr.value.toLowerCase())) rows.push({ path: fr.value, count: fr.count, enabled: true });
    }
    return rows;
  });
  const catCounts = $derived(new Map(search.facets.categories.map((c) => [c.value, c.count])));
  const categories = $derived(CATEGORIES.map((c) => ({ id: c, count: catCounts.get(c) })).filter((c) => search.searched ? c.count || f.categories.includes(c.id) : true));
  const extensions = $derived.by(() => {
    const top = search.facets.extensions.slice(0, 12).map((e) => e.value);
    for (const e of f.extensions) if (!top.includes(e)) top.push(e);
    return top.map((e) => ({ ext: e, count: search.facets.extensions.find((x) => x.value === e)?.count }));
  });

  function fmt(n: number | undefined): string { return n === undefined ? '' : formatNumber(n, lang); }
  function num(v: string): number | null { const n = Number(v); return v === '' || Number.isNaN(n) ? null : n; }
</script>

<aside class="sidebar" aria-label={t('sidebar.filters')}>
  <div class="head">
    <span class="title"><Icon name="filter" size={14} /> {t('sidebar.filters')}</span>
    {#if active}<button class="link" onclick={() => search.resetFilters()}>{t('sidebar.reset')}</button>{/if}
  </div>
  <div class="scroll">
    <section>
      <h3 class="section-title">{t('sidebar.locations')}</h3>
      {#each locations as loc (loc.path)}
        <Checkbox checked={f.roots.includes(loc.path)} count={fmt(loc.count)} title={loc.path}
          onchange={() => search.setFilters({ roots: search.toggleIn(f.roots, loc.path) })}>
          <Icon name="folder" size={15} style="color: var(--fg-3); flex: 0 0 auto" /><span class="ellipsis" class:dim={!loc.enabled}>{baseName(loc.path) || loc.path}</span>
        </Checkbox>
      {/each}
      <button class="btn sm subtle add" onclick={() => actions.addFolders()}><Icon name="folderAdd" size={15} /> {t('sidebar.addFolder')}</button>
    </section>

    <section>
      <h3 class="section-title">{t('sidebar.fileType')}</h3>
      {#if categories.length === 0}
        <div class="faint small">{t('sidebar.noFacets')}</div>
      {:else if categories.length > 1}
        <div class="faint small hint">{t('sidebar.multiSelectHint')}</div>
      {/if}
      {#each categories as c (c.id)}
        <Checkbox checked={f.categories.includes(c.id)} count={fmt(c.count)}
          onchange={() => search.setFilters({ categories: search.toggleIn(f.categories, c.id) })}>
          <Icon name={CATEGORY_ICON[c.id]} size={16} style="color: var(--cat-{c.id}); flex: 0 0 auto" /><span class="ellipsis">{t(`cat.${c.id}`)}</span>
        </Checkbox>
      {/each}
    </section>

    <section>
      <h3 class="section-title">{t('sidebar.extensions')}</h3>
      {#if extensions.length === 0}
        <div class="faint small">{t('sidebar.noFacets')}</div>
      {:else}
        <div class="chips">
          {#each extensions as e (e.ext)}
            <button class="chip" class:on={f.extensions.includes(e.ext)} aria-pressed={f.extensions.includes(e.ext)}
              onclick={() => search.setFilters({ extensions: search.toggleIn(f.extensions, e.ext) })}>
              .{e.ext}{#if e.count !== undefined}<span class="count">{fmt(e.count)}</span>{/if}
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <section>
      <h3 class="section-title">{t('sidebar.modified')}</h3>
      <div class="radios" role="radiogroup" aria-label={t('sidebar.modified')}>
        {#each MODIFIED as m (m)}
          <label class="radio"><input type="radio" name="modified" value={m} checked={f.modified === m} onchange={() => search.setFilters({ modified: m })} /><span class="dot"></span>{t(`sidebar.modified.${m}`)}</label>
        {/each}
      </div>
      {#if f.modified === 'custom'}
        <div class="range">
          <label><span class="faint">{t('sidebar.from')}</span><input class="input sm" type="date" value={f.modifiedFrom} onchange={(e) => search.setFilters({ modifiedFrom: (e.currentTarget as HTMLInputElement).value })} /></label>
          <label><span class="faint">{t('sidebar.to')}</span><input class="input sm" type="date" value={f.modifiedTo} onchange={(e) => search.setFilters({ modifiedTo: (e.currentTarget as HTMLInputElement).value })} /></label>
        </div>
      {/if}
    </section>

    <section>
      <h3 class="section-title">{t('sidebar.size')}</h3>
      <div class="radios" role="radiogroup" aria-label={t('sidebar.size')}>
        {#each SIZES as s (s)}
          <label class="radio"><input type="radio" name="size" value={s} checked={f.size === s} onchange={() => search.setFilters({ size: s })} /><span class="dot"></span>{t(`sidebar.size.${s}`)}</label>
        {/each}
      </div>
      {#if f.size === 'custom'}
        <div class="range">
          <label><span class="faint">{t('sidebar.size.minKb')}</span><input class="input sm" type="number" min="0" value={f.sizeMinKb ?? ''} onchange={(e) => search.setFilters({ sizeMinKb: num((e.currentTarget as HTMLInputElement).value) })} /></label>
          <label><span class="faint">{t('sidebar.size.maxKb')}</span><input class="input sm" type="number" min="0" value={f.sizeMaxKb ?? ''} onchange={(e) => search.setFilters({ sizeMaxKb: num((e.currentTarget as HTMLInputElement).value) })} /></label>
        </div>
      {/if}
    </section>

    <section>
      <Checkbox checked={f.withContentOnly} label={t('sidebar.onlyWithText')} onchange={(v) => search.setFilters({ withContentOnly: v })} />
    </section>
  </div>
</aside>

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--bg); }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 6px 14px 4px 14px; min-height: 32px; }
  .title { display: inline-flex; align-items: center; gap: 6px; font-weight: 600; font-size: var(--fs-md); color: var(--fg-2); }
  .link { color: var(--accent); font-size: var(--fs-sm); padding: 2px 4px; border-radius: 4px; }
  .link:hover { text-decoration: underline; }
  .scroll { flex: 1 1 auto; overflow-y: auto; padding: 0 8px 12px 8px; }
  section { padding: 8px 0 10px; border-bottom: 1px solid var(--divider); }
  section:last-child { border-bottom: 0; }
  .section-title { padding: 0 6px 6px; }
  .add { margin-top: 2px; margin-left: 2px; color: var(--accent); }
  .small { font-size: var(--fs-sm); padding: 2px 6px; }
  .dim { color: var(--fg-3); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; padding: 2px 6px; }
  .radios { display: flex; flex-direction: column; gap: 1px; }
  .radio { display: flex; align-items: center; gap: 8px; min-height: 26px; padding: 0 6px; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--fs-md); }
  .radio:hover { background: var(--surface-hover); }
  .radio input { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .dot { width: 18px; height: 18px; border-radius: 50%; border: 1px solid var(--stroke-control-strong); background: var(--surface); display: grid; place-items: center; flex: 0 0 auto; transition: border-color var(--dur) var(--ease); }
  .dot::after { content: ''; width: 8px; height: 8px; border-radius: 50%; background: var(--accent-fg); transform: scale(0); transition: transform var(--dur) var(--ease); }
  .radio input:checked + .dot { background: var(--accent); border-color: var(--accent); }
  .radio input:checked + .dot::after { transform: scale(1); }
  .radio input:focus-visible + .dot { outline: 2px solid var(--focus); outline-offset: 1px; }
  .range { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; padding: 6px 6px 0; }
  .range label { display: flex; flex-direction: column; gap: 3px; font-size: var(--fs-xs); min-width: 0; }
  .range .input { width: 100%; min-width: 0; padding: 0 6px; }
  .hint { margin: -2px 0 4px; }
</style>

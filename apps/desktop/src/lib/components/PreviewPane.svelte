<script lang="ts">
  import { t } from '../i18n';
  import { search } from '../stores/search.svelte';
  import { settings } from '../stores/settings.svelte';
  import { ui } from '../stores/ui.svelte';
  import { actions } from '../actions';
  import { formatDate, formatNumber, formatSize } from '../utils/format';
  import { dirName } from '../utils/path';
  import Icon, { CATEGORY_ICON } from './Icon.svelte';

  const RENDER_CAP = 300_000;
  let showAll = $state(false);
  let body = $state<HTMLElement | null>(null);

  const hit = $derived(search.selected);
  const preview = $derived(search.preview);
  const lang = $derived(settings.value.uiLanguage);
  const previewPath = $derived(preview?.path);
  $effect(() => { void previewPath; showAll = false; });

  /** Segments to render, capped for huge documents; match indices are assigned to highlighted segments. */
  const rendered = $derived.by(() => {
    if (!preview) return { segs: [] as Array<{ text: string; highlight: boolean; matchIndex: number }>, capped: false, shownChars: 0 };
    const out: Array<{ text: string; highlight: boolean; matchIndex: number }> = [];
    let chars = 0;
    let mi = 0;
    let capped = false;
    for (const s of preview.segments) {
      if (!showAll && chars >= RENDER_CAP) { capped = true; break; }
      let text = s.text;
      if (!showAll && chars + text.length > RENDER_CAP) { text = text.slice(0, RENDER_CAP - chars); capped = true; }
      out.push({ text, highlight: s.highlight, matchIndex: s.highlight ? mi++ : -1 });
      chars += text.length;
      if (capped) break;
    }
    return { segs: out, capped, shownChars: chars };
  });

  // Scroll the current match into view whenever it changes (or the preview loads).
  $effect(() => {
    const i = search.currentMatch;
    void preview;
    if (!body) return;
    requestAnimationFrame(() => {
      const el = body?.querySelector<HTMLElement>(`mark[data-i="${i}"]`);
      el?.scrollIntoView({ block: 'center' });
    });
  });

  function matchLabel(): string {
    const n = preview?.totalMatches ?? 0;
    return n === 0 ? t('preview.noMatches') : `${search.currentMatch + 1} / ${formatNumber(n, lang)}`;
  }
</script>

<aside class="preview" aria-label={t('preview.title')}>
  {#if hit}
    <header class="head">
      <div class="title-row">
        <span class="ficon" style="color: var(--cat-{hit.category})"><Icon name={CATEGORY_ICON[hit.category] ?? 'fileOther'} size={22} /></span>
        <h2 class="name ellipsis selectable" title={hit.name}>{hit.name}</h2>
        <button class="btn xs icon subtle" onclick={() => ui.togglePreview()} title="{t('preview.collapse')} (Ctrl+P)" aria-label={t('preview.collapse')}><Icon name="close" size={14} /></button>
      </div>
      <div class="path ellipsis selectable" title={hit.path}>{dirName(hit.path)}</div>
      <div class="meta">
        <span class="tabular">{formatSize(hit.size, lang)}</span>
        <span class="sep">·</span>
        <span class="tabular">{formatDate(hit.modified, lang)}</span>
        {#if preview?.encoding ?? hit.encoding}
          <span class="sep">·</span><span>{preview?.encoding ?? hit.encoding}</span>
        {/if}
        {#if preview}
          <span class="sep">·</span><span class="tabular">{t('preview.matches', { count: formatNumber(preview.totalMatches, lang) })}</span>
          <span class="sep">·</span><span class="faint">{t(`preview.source.${preview.source}`)}</span>
        {/if}
      </div>
      <div class="tools">
        <button class="btn sm" onclick={() => actions.open(hit)}><Icon name="open" size={14} /> {t('results.open')}</button>
        <button class="btn sm" onclick={() => actions.reveal(hit)}><Icon name="explorer" size={14} /> {t('results.reveal')}</button>
        <button class="btn sm" onclick={() => actions.copyPath(hit)}><Icon name="copy" size={14} /> {t('results.copyPath')}</button>
        <span class="grow"></span>
        <div class="nav-group">
        <div class="nav" class:disabled={!preview || preview.totalMatches === 0}>
          <button class="btn sm icon subtle" onclick={() => search.nextMatch(-1)} title={t('preview.prev')} aria-label={t('preview.prev')} disabled={!preview || preview.totalMatches === 0}><Icon name="chevronLeft" size={14} /></button>
          <span class="count tabular">{matchLabel()}</span>
          <button class="btn sm icon subtle" onclick={() => search.nextMatch(1)} title={t('preview.next')} aria-label={t('preview.next')} disabled={!preview || preview.totalMatches === 0}><Icon name="chevronRight" size={14} /></button>
        </div>
        <button class="btn sm icon subtle" class:on={ui.previewMono} onclick={() => ui.setPreviewMono(!ui.previewMono)} title={t('preview.mono')} aria-label={t('preview.mono')} aria-pressed={ui.previewMono}><Icon name="mono" size={15} /></button>
        </div>
      </div>
    </header>

    <div class="body selectable" class:mono={ui.previewMono} bind:this={body}>
      {#if search.previewLoading && !preview}
        <div class="sk" aria-busy="true">
          {#each [92, 70, 85, 40, 0, 78, 95, 60, 88, 30] as w, i (i)}
            {#if w === 0}<div class="sk-gap"></div>{:else}<div class="skeleton sk-l" style="width: {w}%"></div>{/if}
          {/each}
        </div>
      {:else if search.previewError}
        <div class="notice err"><Icon name="error" size={16} /> {t('preview.error', { error: search.previewError })}</div>
      {:else if preview}
        {#if preview.chars === 0}
          <div class="notice"><Icon name="info" size={16} /> {t('preview.noContent')}</div>
        {:else}
          {#if rendered.capped}
            <div class="notice"><Icon name="info" size={16} /> {t('preview.truncated', { chars: formatNumber(rendered.shownChars, lang) })} <button class="link" onclick={() => (showAll = true)}>{t('preview.showAll')}</button></div>
          {/if}
          <pre class="text" class:dim={search.previewLoading}>{#each rendered.segs as s, i (i)}{#if s.highlight}<mark data-i={s.matchIndex} class:current={s.matchIndex === search.currentMatch}>{s.text}</mark>{:else}{s.text}{/if}{/each}</pre>
          {#if preview.truncated}
            <div class="notice"><Icon name="info" size={16} /> {t('preview.truncated', { chars: formatNumber(preview.chars, lang) })}</div>
          {/if}
        {/if}
      {/if}
    </div>
  {:else}
    <div class="placeholder">
      <Icon name="eye" size={36} />
      <p>{t('preview.empty')}</p>
    </div>
  {/if}
</aside>

<style>
  .preview { height: 100%; min-height: 0; display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--stroke); border-radius: var(--radius); overflow: hidden; }
  .head { padding: 12px 14px 10px; border-bottom: 1px solid var(--divider); display: flex; flex-direction: column; gap: 4px; background: var(--surface-2); }
  .title-row { display: flex; align-items: center; gap: 8px; }
  .ficon { flex: 0 0 auto; }
  .name { flex: 1 1 auto; min-width: 0; font-size: var(--fs-lg); font-weight: 600; }
  .path { font-size: var(--fs-sm); color: var(--fg-3); padding-left: 30px; }
  .meta { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; font-size: var(--fs-sm); color: var(--fg-2); padding-left: 30px; }
  .sep { color: var(--fg-3); }
  .tools { display: flex; align-items: center; gap: 6px; margin-top: 8px; flex-wrap: wrap; }
  .grow { flex: 1 1 auto; }
  .nav-group { display: inline-flex; align-items: center; gap: 6px; }
  .nav { display: inline-flex; align-items: center; gap: 2px; border: 1px solid var(--stroke-control); border-radius: var(--radius-sm); padding: 0 2px; height: 28px; background: var(--surface); }
  .nav.disabled { color: var(--fg-disabled); }
  .count { min-width: 56px; text-align: center; font-size: var(--fs-sm); padding: 0 4px; }
  .on { background: var(--accent-soft); color: var(--accent); }
  .body { flex: 1 1 auto; min-height: 0; overflow: auto; padding: 12px 16px 24px; font-size: var(--fs-md); line-height: 1.55; }
  .body.mono { font: 13px/1.5 var(--font-mono); }
  .text { margin: 0; white-space: pre-wrap; word-break: break-word; font: inherit; transition: opacity var(--dur) var(--ease); }
  .text.dim { opacity: 0.55; }
  .text mark { padding: 0 1px; scroll-margin: 80px; }
  .sk { display: flex; flex-direction: column; gap: 10px; padding-top: 4px; }
  .sk-l { height: 12px; }
  .sk-gap { height: 8px; }
  .notice { display: flex; align-items: center; gap: 8px; padding: 8px 10px; margin-bottom: 10px; border-radius: var(--radius-sm); background: var(--accent-soft); color: var(--fg-2); font-size: var(--fs-sm); font-family: var(--font-ui); }
  .notice.err { background: var(--danger-soft); color: var(--danger); }
  .link { color: var(--accent); }
  .link:hover { text-decoration: underline; }
  .placeholder { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--fg-3); text-align: center; padding: 24px; }
</style>

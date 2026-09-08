<script lang="ts">
  import type { SearchHit } from '../api/types';
  import { settings } from '../stores/settings.svelte';
  import { search } from '../stores/search.svelte';
  import { formatDate, formatSize } from '../utils/format';
  import { dirName, ellipsizePath } from '../utils/path';
  import { highlightText, sanitizeSnippet } from '../utils/highlight';
  import { t } from '../i18n';
  import Icon, { CATEGORY_ICON } from './Icon.svelte';

  interface Props {
    hit: SearchHit;
    index: number;
    selected: boolean;
    onselect: (index: number) => void;
    onopen: (hit: SearchHit) => void;
    oncontextmenu: (e: MouseEvent, hit: SearchHit) => void;
  }
  let { hit, index, selected, onselect, onopen, oncontextmenu }: Props = $props();
  const lang = $derived(settings.value.uiLanguage);
  const nameHtml = $derived(highlightText(hit.name, search.query, search.mode));
  const snippetHtml = $derived(sanitizeSnippet(hit.snippet));
  const dir = $derived(dirName(hit.path));
</script>

<div
  class="hit"
  class:selected
  role="option"
  aria-selected={selected}
  tabindex="-1"
  data-index={index}
  onmousedown={(e) => { if (e.button !== 1) onselect(index); }}
  ondblclick={() => onopen(hit)}
  oncontextmenu={(e) => { onselect(index); oncontextmenu(e, hit); }}
>
  <span class="ficon" style="color: var(--cat-{hit.category})" title={t(`cat.${hit.category}`)}><Icon name={CATEGORY_ICON[hit.category] ?? 'fileOther'} size={26} /></span>
  <div class="main">
    <div class="line1">
      <span class="name ellipsis">{@html nameHtml}</span>
      {#if !hit.hasContent}<span class="badge">{t('results.nameOnly')}</span>{/if}
    </div>
    <div class="path ellipsis" title={hit.path}>{ellipsizePath(dir, 80)}</div>
    {#if snippetHtml}
      <div class="snippet">{@html snippetHtml}</div>
    {/if}
  </div>
  <div class="meta tabular">
    <span class="size">{formatSize(hit.size, lang)}</span>
    <span class="date">{formatDate(hit.modified, lang)}</span>
  </div>
</div>

<style>
  .hit {
    display: flex; gap: 12px; align-items: flex-start; height: 100%; padding: 8px 12px 0 12px; margin: 0 8px;
    border-radius: var(--radius); cursor: default; position: relative;
    transition: background var(--dur) var(--ease);
  }
  .hit:hover { background: var(--surface-hover); }
  .hit.selected { background: var(--surface-selected); }
  .hit.selected:hover { background: var(--surface-selected-hover); }
  .hit.selected::before { content: ''; position: absolute; left: 0; top: 14px; bottom: 14px; width: 3px; border-radius: 2px; background: var(--accent); }
  .ficon { flex: 0 0 auto; margin-top: 2px; }
  .main { flex: 1 1 auto; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .line1 { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .name { font-weight: 600; font-size: var(--fs-lg); color: var(--fg); }
  .badge { flex: 0 0 auto; font-size: var(--fs-xs); color: var(--fg-3); border: 1px solid var(--stroke-strong); border-radius: 3px; padding: 0 5px; line-height: 15px; }
  .path { font-size: var(--fs-sm); color: var(--fg-3); }
  .snippet {
    font-size: var(--fs-sm); color: var(--fg-2); line-height: 1.45; margin-top: 2px;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }
  .meta { flex: 0 0 auto; display: flex; flex-direction: column; align-items: flex-end; gap: 2px; font-size: var(--fs-sm); color: var(--fg-3); padding-top: 3px; min-width: 76px; }
</style>

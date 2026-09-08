<script lang="ts">
  import { t, i18n } from '../i18n';
  import type { UiLanguage } from '../api/types';
  import { search } from '../stores/search.svelte';
  import * as api from '../api/backend';
  import Icon from './Icon.svelte';

  interface Props { onclose: () => void }
  let { onclose }: Props = $props();

  let tokens = $state<string[] | null>(null);
  let el = $state<HTMLElement | null>(null);

  const EXAMPLES: Record<UiLanguage, string[]> = {
    ru: ['"договор аренды"', 'договор OR шарт', '-черновик', 'аренд*', 'договор~', 'name:отчёт', 'ext:xlsx', 'path:Договоры', 'content:аренда'],
    kk: ['"жалдау шарты"', 'шарт OR договор', '-жоба', 'жалда*', 'шарт~', 'name:есеп', 'ext:xlsx', 'path:Келісімшарттар', 'content:жалдау'],
    en: ['"lease agreement"', 'lease OR rent', '-draft', 'agree*', 'lease~', 'name:report', 'ext:xlsx', 'path:Contracts', 'content:lease']
  };
  const KEYS = ['search.help.phrase', 'search.help.or', 'search.help.exclude', 'search.help.prefix', 'search.help.fuzzy', 'search.help.name', 'search.help.ext', 'search.help.path', 'search.help.content'] as const;
  const syntax = $derived(EXAMPLES[i18n.lang].map((ex, idx) => ({ ex, key: KEYS[idx] })));
  const shortcuts = [
    { keys: ['Ctrl', 'F'], key: 'keys.focusSearch' as const },
    { keys: ['↑', '↓'], key: 'keys.navigate' as const },
    { keys: ['Enter'], key: 'keys.openFile' as const },
    { keys: ['Ctrl', 'Enter'], key: 'keys.reveal' as const },
    { keys: ['Ctrl', 'C'], key: 'keys.copyPath' as const },
    { keys: ['F3'], key: 'keys.nextMatch' as const },
    { keys: ['F5'], key: 'keys.updateIndex' as const },
    { keys: ['Ctrl', 'B'], key: 'keys.toggleSidebar' as const },
    { keys: ['Ctrl', 'P'], key: 'keys.togglePreview' as const },
    { keys: ['Ctrl', ','], key: 'keys.settings' as const },
    { keys: ['Esc'], key: 'keys.escape' as const }
  ];

  $effect(() => {
    const q = search.query.trim();
    const mode = search.mode;
    if (!q) { tokens = null; return; }
    let alive = true;
    api.analyzeText(q, mode).then((r) => { if (alive) tokens = r; }).catch(() => { if (alive) tokens = null; });
    return () => { alive = false; };
  });

  $effect(() => {
    const onDown = (e: PointerEvent) => { if (el && !el.contains(e.target as Node)) onclose(); };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); onclose(); } };
    document.addEventListener('pointerdown', onDown, true);
    document.addEventListener('keydown', onKey, true);
    return () => {
      document.removeEventListener('pointerdown', onDown, true);
      document.removeEventListener('keydown', onKey, true);
    };
  });
</script>

<div class="flyout help" bind:this={el} role="dialog" aria-label={t('search.help.title')}>
  <div class="head">
    <h3>{t('search.help.title')}</h3>
    <button class="btn xs icon subtle" onclick={onclose} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
  </div>
  <div class="cols">
    <dl class="syntax">
      {#each syntax as s (s.ex)}
        <dt><code>{s.ex}</code></dt><dd>{t(s.key)}</dd>
      {/each}
    </dl>
    <div class="keys">
      <div class="section-title">{t('search.help.shortcuts')}</div>
      <ul>
        {#each shortcuts as s (s.key)}
          <li><span class="combo">{#each s.keys as k, i (k)}{#if i > 0}<span class="plus">+</span>{/if}<span class="kbd">{k}</span>{/each}</span><span class="desc">{t(s.key)}</span></li>
        {/each}
      </ul>
    </div>
  </div>
  {#if tokens && tokens.length}
    <div class="analyze">
      <span class="section-title">{t('search.help.analyze')}</span>
      <div class="tokens">{#each tokens as tk, i (i)}<span class="chip on">{tk}</span>{/each}</div>
    </div>
  {/if}
</div>

<style>
  .help { position: absolute; top: calc(100% + 6px); left: 0; z-index: 60; width: 640px; max-width: calc(100vw - 32px); padding: 14px 16px 12px; animation: fade-in var(--dur) var(--ease); }
  .head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  h3 { font-size: var(--fs-lg); font-weight: 600; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .syntax { display: grid; grid-template-columns: auto 1fr; gap: 4px 12px; margin: 0; align-items: center; }
  .syntax dt code { font-family: var(--font-mono); font-size: var(--fs-sm); background: var(--surface-3); padding: 2px 6px; border-radius: 4px; white-space: nowrap; }
  .syntax dd { margin: 0; color: var(--fg-2); font-size: var(--fs-sm); }
  .keys ul { display: flex; flex-direction: column; gap: 3px; margin-top: 6px; }
  .keys li { display: flex; align-items: center; gap: 10px; font-size: var(--fs-sm); }
  .combo { display: inline-flex; align-items: center; gap: 2px; min-width: 96px; }
  .plus { color: var(--fg-3); padding: 0 1px; }
  .desc { color: var(--fg-2); }
  .analyze { margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--divider); display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .tokens { display: flex; gap: 6px; flex-wrap: wrap; }
</style>

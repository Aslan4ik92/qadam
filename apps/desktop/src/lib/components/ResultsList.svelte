<script lang="ts">
  import { t } from '../i18n';
  import { search } from '../stores/search.svelte';
  import { indexing } from '../stores/indexing.svelte';
  import { settings } from '../stores/settings.svelte';
  import { actions } from '../actions';
  import { formatNumber } from '../utils/format';
  import VirtualList from './VirtualList.svelte';
  import ResultRow from './ResultRow.svelte';
  import Onboarding from './Onboarding.svelte';
  import Icon from './Icon.svelte';
  import type { SearchHit } from '../api/types';

  const ROW_HEIGHT = 84;
  let list = $state<ReturnType<typeof VirtualList<SearchHit>> | null>(null);
  const lang = $derived(settings.value.uiLanguage);

  // Keep the selected row visible when selection changes via keyboard.
  $effect(() => {
    const i = search.selectedIndex;
    if (i >= 0) list?.scrollToIndex(i);
  });

  const view = $derived.by<'onboarding' | 'start' | 'loading' | 'error' | 'empty' | 'indexing-empty' | 'results'>(() => {
    if (settings.loaded && !settings.hasRoots && !search.searched) return 'onboarding';
    if (search.error) return 'error';
    if (search.hits.length > 0) return 'results';
    if (search.loading) return 'loading';
    if (!search.active || !search.searched) return 'start';
    if (indexing.running) return 'indexing-empty';
    return 'empty';
  });
</script>

<section class="results" aria-label={t('results.open')}>
  {#if view === 'onboarding'}
    <Onboarding />
  {:else if view === 'results'}
    <VirtualList bind:this={list} items={search.hits} rowHeight={ROW_HEIGHT} key={(h) => h.path} onnearend={() => search.loadMore()} class="list">
      {#snippet row(hit, index)}
        <ResultRow {hit} {index} selected={index === search.selectedIndex}
          onselect={(i) => search.select(i)} onopen={(h) => actions.open(h)} oncontextmenu={(e, h) => actions.contextMenu(e, h)} />
      {/snippet}
      {#snippet footer()}
        <div class="footer">
          <span class="faint tabular">{t('results.shown', { shown: formatNumber(search.hits.length, lang), total: formatNumber(search.total, lang) })}</span>
          {#if search.hasMore}
            <button class="btn sm" onclick={() => search.loadMore()} disabled={search.loadingMore}>
              {#if search.loadingMore}{t('common.loading')}{:else}{t('results.loadMore')}{/if}
            </button>
          {/if}
        </div>
      {/snippet}
    </VirtualList>
  {:else if view === 'loading'}
    <div class="skeletons" aria-busy="true">
      {#each [0, 1, 2, 3, 4, 5] as i (i)}
        <div class="sk-row"><div class="skeleton sk-ic"></div><div class="sk-lines"><div class="skeleton sk-l" style="width: 45%"></div><div class="skeleton sk-l" style="width: 70%"></div><div class="skeleton sk-l" style="width: 90%"></div></div></div>
      {/each}
    </div>
  {:else}
    <div class="empty">
      {#if view === 'error'}
        <span class="eicon err"><Icon name="error" size={40} /></span>
        <h2>{t('toast.error')}</h2>
        <p class="selectable">{t('results.error', { error: search.error ?? '' })}</p>
      {:else if view === 'empty'}
        <span class="eicon"><Icon name="search" size={40} /></span>
        <h2>{t('results.empty.title')}</h2>
        <p>{t('results.empty.text')}</p>
        <ul class="tips">
          <li>{t('results.empty.tip1')}</li>
          <li>{t('results.empty.tip2')}</li>
          <li>{t('results.empty.tip3')}</li>
        </ul>
      {:else if view === 'indexing-empty'}
        <span class="eicon spin"><Icon name="refresh" size={40} /></span>
        <h2>{t('results.indexing.title')}</h2>
        <p>{t('results.indexing.text')}</p>
      {:else}
        <span class="eicon"><Icon name="logo" size={48} /></span>
        <h2>{t('results.start.title')}</h2>
        <p>{t('results.start.text')}</p>
        {#if indexing.stats && indexing.stats.documents === 0 && !indexing.running}
          <button class="btn primary" onclick={() => indexing.start(false)}><Icon name="refresh" size={15} /> {t('index.update')}</button>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .results { height: 100%; min-height: 0; display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--stroke); border-radius: var(--radius); overflow: hidden; }
  .results :global(.list) { padding: 6px 0; }
  .footer { display: flex; align-items: center; justify-content: center; gap: 12px; padding: 12px; font-size: var(--fs-sm); }
  .skeletons { padding: 12px 20px; display: flex; flex-direction: column; gap: 18px; }
  .sk-row { display: flex; gap: 12px; }
  .sk-ic { width: 28px; height: 32px; }
  .sk-lines { flex: 1; display: flex; flex-direction: column; gap: 8px; }
  .sk-l { height: 12px; }
  .empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 40px; gap: 8px; color: var(--fg-2); }
  .eicon { color: var(--fg-3); margin-bottom: 8px; }
  .eicon.err { color: var(--danger); }
  .empty h2 { font-size: var(--fs-xl); font-weight: 600; color: var(--fg); }
  .empty p { max-width: 460px; }
  .tips { margin-top: 10px; text-align: left; max-width: 460px; color: var(--fg-3); font-size: var(--fs-sm); display: flex; flex-direction: column; gap: 6px; padding-left: 16px; list-style: disc; }
  .empty .btn { margin-top: 10px; }
  .spin :global(svg) { animation: rot 1.6s linear infinite; }
  @keyframes rot { to { transform: rotate(360deg); } }
</style>

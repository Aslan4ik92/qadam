<script lang="ts">
  import { t } from '../i18n';
  import { indexing } from '../stores/indexing.svelte';
  import { settings } from '../stores/settings.svelte';
  import { ui } from '../stores/ui.svelte';
  import * as api from '../api/backend';
  import { errorMessage } from '../api/backend';
  import { formatNumber, formatRelative, formatSize } from '../utils/format';
  import { ellipsizePath } from '../utils/path';
  import Icon from './Icon.svelte';
  import type { AppInfo } from '../api/types';

  const lang = $derived(settings.value.uiLanguage);
  const p = $derived(indexing.progress);
  const stats = $derived(indexing.stats);
  let info = $state<AppInfo | null>(null);
  let now = $state(Date.now());

  $effect(() => {
    api.getAppInfo().then((i) => (info = i)).catch(() => { /* version stays hidden */ });
    const tm = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(tm);
  });

  const progressText = $derived.by(() => {
    switch (p.phase) {
      case 'scanning': return `${t('status.phase.scanning')} ${formatNumber(p.scanned, lang)}`;
      case 'extracting': {
        const speed = indexing.speed > 0 ? ` · ${t('status.speed', { speed: formatSize(indexing.speed, lang) })}` : '';
        return `${t('status.phase.extracting')} ${formatNumber(p.indexed, lang)} / ${formatNumber(p.queued, lang)}${speed}`;
      }
      default: return t(`status.phase.${p.phase}`);
    }
  });

  async function openLogs() {
    try { await api.openLogsFolder(); } catch (e) { ui.error(t('toast.error'), errorMessage(e)); }
  }
</script>

<footer class="status">
  <div class="left">
    {#if p.running}
      <span class="phase">{progressText}</span>
      <div class="bar" class:indeterminate={indexing.fraction === null} role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={indexing.fraction === null ? undefined : Math.round(indexing.fraction * 100)}>
        <div class="fill" style:width={indexing.fraction === null ? undefined : `${Math.round(indexing.fraction * 100)}%`}></div>
      </div>
      {#if p.current}<span class="current ellipsis" title={p.current}>{ellipsizePath(p.current, 70)}</span>{/if}
      <button class="btn xs" onclick={() => indexing.cancel()}><Icon name="stop" size={11} /> {t('index.cancel')}</button>
    {:else}
      <span class="dot" class:on={stats?.watching} title={stats?.watching ? t('status.watching') : t('status.notWatching')}></span>
      <span class="idx">
        {t('status.index')}:
        {#if stats}
          <span class="tabular">{t('status.files', { count: formatNumber(stats.documents, lang) })}</span>
          <span class="sep">·</span>
          {#if stats.lastIndexed}{t('status.updated', { ago: formatRelative(stats.lastIndexed, lang, now, t('time.justNow')) })}{:else}{t('status.never')}{/if}
        {:else if indexing.statsError}
          <span class="err" title={indexing.statsError}>{t('toast.error')}</span>
        {:else}
          <span class="faint">{t('common.loading')}</span>
        {/if}
      </span>
      {#if p.phase === 'failed' && p.error}
        <span class="err ellipsis" title={p.error}>· {t('status.phase.failed')}: {p.error}</span>
      {/if}
    {/if}
  </div>
  <div class="right">
    {#if info}<span class="faint">{t('status.version')} {info.version}</span>{/if}
    <button class="link" onclick={openLogs}><Icon name="logs" size={13} /> {t('status.logs')}</button>
  </div>
</footer>

<style>
  .status { height: 28px; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 14px; font-size: var(--fs-sm); color: var(--fg-2); background: var(--bg); border-top: 1px solid var(--divider); }
  .left { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1 1 auto; }
  .right { display: flex; align-items: center; gap: 12px; flex: 0 0 auto; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--fg-disabled); flex: 0 0 auto; }
  .dot.on { background: var(--success); box-shadow: 0 0 0 3px var(--success-soft); }
  .idx { white-space: nowrap; }
  .sep { color: var(--fg-3); margin: 0 2px; }
  .phase { white-space: nowrap; font-weight: 600; color: var(--fg); }
  .bar { width: 160px; height: 4px; border-radius: 2px; background: var(--surface-3); overflow: hidden; flex: 0 0 auto; position: relative; }
  .fill { height: 100%; background: var(--accent); border-radius: 2px; transition: width 250ms linear; }
  .bar.indeterminate .fill { width: 40%; position: absolute; animation: indet 1.4s var(--ease) infinite; }
  @keyframes indet { from { left: -40%; } to { left: 100%; } }
  .current { color: var(--fg-3); min-width: 0; flex: 1 1 auto; max-width: 520px; }
  .err { color: var(--danger); }
  .link { display: inline-flex; align-items: center; gap: 4px; color: var(--accent); padding: 2px 4px; border-radius: 4px; }
  .link:hover { text-decoration: underline; }
</style>

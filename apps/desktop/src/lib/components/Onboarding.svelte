<script lang="ts">
  import { t } from '../i18n';
  import * as api from '../api/backend';
  import { errorMessage } from '../api/backend';
  import { settings } from '../stores/settings.svelte';
  import { indexing } from '../stores/indexing.svelte';
  import { ui } from '../stores/ui.svelte';
  import { actions } from '../actions';
  import Icon from './Icon.svelte';
  import Checkbox from './Checkbox.svelte';
  import type { DriveInfo } from '../api/types';

  let drives = $state<DriveInfo[]>([]);
  let chosen = $state<string[]>([]);
  let busy = $state(false);

  $effect(() => {
    api.listDrives().then((d) => (drives = d)).catch(() => (drives = []));
  });

  async function addChosen() {
    if (chosen.length === 0) return;
    busy = true;
    try {
      const added = await settings.addRoots(chosen);
      if (added > 0) {
        chosen = [];
        await indexing.start(false);
      }
    } catch (e) {
      ui.error(t('toast.error'), errorMessage(e));
    } finally { busy = false; }
  }
</script>

<div class="wrap">
  <div class="card onboarding">
    <div class="art"><Icon name="logo" size={56} /></div>
    <h2>{t('onboarding.title')}</h2>
    <p>{t('onboarding.text')}</p>
    <div class="actions">
      <button class="btn primary big" onclick={() => actions.addFolders()} disabled={busy}><Icon name="folderAdd" size={18} /> {t('onboarding.addFolders')}</button>
    </div>
    {#if drives.length}
      <div class="drives">
        <div class="section-title">{t('onboarding.orDrives')}</div>
        <ul>
          {#each drives as d (d.path)}
            <li>
              <Checkbox checked={chosen.includes(d.path)} onchange={(v) => (chosen = v ? [...chosen, d.path] : chosen.filter((x) => x !== d.path))}>
                <Icon name="drive" size={16} style="color: var(--fg-3); flex: 0 0 auto" /><span>{d.label}</span><span class="faint">{d.path}</span>
              </Checkbox>
            </li>
          {/each}
        </ul>
        <button class="btn" onclick={addChosen} disabled={chosen.length === 0 || busy}><Icon name="refresh" size={15} /> {t('onboarding.startIndexing')}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap { flex: 1; display: flex; align-items: center; justify-content: center; padding: 32px; overflow: auto; }
  .onboarding { width: 520px; max-width: 100%; padding: 32px 36px; display: flex; flex-direction: column; align-items: center; text-align: center; gap: 12px; animation: fade-in 200ms var(--ease); }
  .art { margin-bottom: 4px; }
  h2 { font-size: var(--fs-title); font-weight: 600; }
  p { color: var(--fg-2); line-height: 1.5; max-width: 440px; }
  .actions { margin-top: 6px; }
  .big { height: 38px; padding: 0 20px; font-size: var(--fs-lg); }
  .drives { width: 100%; margin-top: 12px; padding-top: 16px; border-top: 1px solid var(--divider); display: flex; flex-direction: column; gap: 8px; align-items: stretch; text-align: left; }
  .drives ul { display: flex; flex-direction: column; gap: 2px; }
  .drives .btn { align-self: flex-start; margin-top: 4px; }
</style>

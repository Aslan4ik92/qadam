<script lang="ts">
  import { ui } from '../stores/ui.svelte';
  import Icon from './Icon.svelte';
  import { t } from '../i18n';

  const timers = new Map<number, ReturnType<typeof setTimeout>>();
  $effect(() => {
    for (const toast of ui.toasts) {
      if (!timers.has(toast.id)) {
        timers.set(toast.id, setTimeout(() => { ui.dismissToast(toast.id); timers.delete(toast.id); }, toast.timeout));
      }
    }
  });
</script>

<div class="toasts" aria-live="polite">
  {#each ui.toasts as toast (toast.id)}
    <div class="toast {toast.kind}" role="status">
      <span class="ic"><Icon name={toast.kind === 'error' ? 'error' : toast.kind === 'success' ? 'success' : 'info'} size={18} /></span>
      <div class="body">
        <div class="title">{toast.title}</div>
        {#if toast.text}<div class="text selectable">{toast.text}</div>{/if}
        {#if toast.actions?.length}
          <div class="actions">
            {#each toast.actions as a (a.label)}
              <button class="btn sm" class:primary={a.primary} onclick={() => { a.run(); ui.dismissToast(toast.id); }}>{a.label}</button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="btn xs icon subtle" onclick={() => ui.dismissToast(toast.id)} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; right: 16px; bottom: 44px; display: flex; flex-direction: column; gap: 8px; z-index: 200; width: 360px; max-width: calc(100vw - 32px); }
  .toast {
    display: flex; gap: 10px; align-items: flex-start; padding: 10px 10px 10px 12px;
    background: var(--surface); border: 1px solid var(--stroke); border-radius: var(--radius); box-shadow: var(--shadow-flyout);
    animation: fade-in var(--dur) var(--ease);
  }
  .ic { flex: 0 0 auto; margin-top: 1px; color: var(--accent); }
  .toast.error .ic { color: var(--danger); }
  .toast.success .ic { color: var(--success); }
  .body { flex: 1 1 auto; min-width: 0; }
  .title { font-weight: 600; font-size: var(--fs-md); }
  .text { color: var(--fg-2); font-size: var(--fs-sm); margin-top: 2px; word-break: break-word; }
  .actions { display: flex; gap: 8px; margin-top: 8px; }
</style>

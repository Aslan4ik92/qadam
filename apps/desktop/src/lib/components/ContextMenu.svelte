<script lang="ts">
  import { ui } from '../stores/ui.svelte';
  import Icon from './Icon.svelte';

  let el = $state<HTMLElement | null>(null);
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    const m = ui.contextMenu;
    if (!m || !el) return;
    // Keep the menu inside the viewport.
    const r = el.getBoundingClientRect();
    const x = Math.min(m.x, window.innerWidth - r.width - 8);
    const y = Math.min(m.y, window.innerHeight - r.height - 8);
    pos = { x: Math.max(4, x), y: Math.max(4, y) };
    el.querySelector<HTMLElement>('button')?.focus();
    const onDown = (e: PointerEvent) => { if (el && !el.contains(e.target as Node)) ui.closeContextMenu(); };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.stopPropagation(); ui.closeContextMenu(); return; }
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        const items = [...(el?.querySelectorAll<HTMLElement>('button') ?? [])];
        const i = items.indexOf(document.activeElement as HTMLElement);
        const n = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
        items[n]?.focus();
        e.preventDefault();
      }
    };
    document.addEventListener('pointerdown', onDown, true);
    document.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', ui.closeContextMenu);
    return () => {
      document.removeEventListener('pointerdown', onDown, true);
      document.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', ui.closeContextMenu);
    };
  });
</script>

{#if ui.contextMenu}
  <div class="flyout ctx" bind:this={el} role="menu" style:left="{pos.x}px" style:top="{pos.y}px" oncontextmenu={(e) => e.preventDefault()}>
    {#each ui.contextMenu.items as item (item.id)}
      {#if item.separatorBefore}<div class="menu-sep"></div>{/if}
      <button class="menu-item" role="menuitem" onclick={() => { const run = item.run; ui.closeContextMenu(); run(); }}>
        <span class="mi">{#if item.icon}<Icon name={item.icon} size={16} />{/if}</span>
        <span>{item.label}</span>
        {#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .ctx { position: fixed; z-index: 150; min-width: 220px; animation: fade-in var(--dur) var(--ease); }
  .mi { width: 16px; display: inline-flex; color: var(--fg-2); }
</style>

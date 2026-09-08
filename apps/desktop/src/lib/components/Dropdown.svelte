<script lang="ts">
  /** Generic flyout anchored to a trigger. The trigger is rendered via the `trigger` snippet, the menu via `children`. */
  import type { Snippet } from 'svelte';
  interface Props {
    open?: boolean;
    align?: 'left' | 'right';
    width?: number;
    trigger: Snippet<[{ open: boolean; toggle: () => void }]>;
    children: Snippet<[{ close: () => void }]>;
    onopenchange?: (open: boolean) => void;
  }
  let { open = $bindable(false), align = 'left', width, trigger, children, onopenchange }: Props = $props();
  let root = $state<HTMLElement | null>(null);
  let menu = $state<HTMLElement | null>(null);

  function setOpen(v: boolean) { if (open !== v) { open = v; onopenchange?.(v); } }
  function toggle() { setOpen(!open); }
  function close() { setOpen(false); }

  function onDocPointer(e: PointerEvent) {
    if (root && !root.contains(e.target as Node)) close();
  }
  function onDocKey(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.stopPropagation(); close(); return; }
    if (!menu) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const items = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"], [role="menuitemradio"], button:not([disabled])')];
      if (items.length === 0) return;
      e.preventDefault();
      const i = items.indexOf(document.activeElement as HTMLElement);
      const next = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
      items[next].focus();
    }
  }
  $effect(() => {
    if (!open) return;
    document.addEventListener('pointerdown', onDocPointer, true);
    document.addEventListener('keydown', onDocKey, true);
    queueMicrotask(() => menu?.querySelector<HTMLElement>('[role="menuitem"], [role="menuitemradio"], button')?.focus());
    return () => {
      document.removeEventListener('pointerdown', onDocPointer, true);
      document.removeEventListener('keydown', onDocKey, true);
    };
  });
</script>

<div class="dd" bind:this={root}>
  {@render trigger({ open, toggle })}
  {#if open}
    <div class="flyout menu" class:right={align === 'right'} bind:this={menu} role="menu" style:min-width={width ? `${width}px` : undefined}>
      {@render children({ close })}
    </div>
  {/if}
</div>

<style>
  .dd { position: relative; display: inline-flex; }
  .menu { position: absolute; top: calc(100% + 4px); left: 0; z-index: 50; animation: fade-in var(--dur) var(--ease); }
  .menu.right { left: auto; right: 0; }
</style>

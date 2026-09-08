<script lang="ts" generics="T">
  /**
   * Fixed-row-height virtual list: renders only the rows visible in the scroll viewport (+ overscan).
   * Exposes `scrollToIndex` so keyboard selection stays visible.
   */
  import type { Snippet } from 'svelte';
  interface Props {
    items: T[];
    rowHeight: number;
    overscan?: number;
    row: Snippet<[T, number]>;
    footer?: Snippet;
    onnearend?: () => void;
    key?: (item: T, index: number) => string | number;
    class?: string;
  }
  let { items, rowHeight, overscan = 6, row, footer, onnearend, key, class: cls = '' }: Props = $props();

  let viewport = $state<HTMLElement | null>(null);
  let scrollTop = $state(0);
  let height = $state(600);

  const total = $derived(items.length * rowHeight);
  const start = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - overscan));
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + overscan));
  const visible = $derived(items.slice(start, end).map((item, i) => ({ item, index: start + i })));

  function onScroll() {
    if (!viewport) return;
    scrollTop = viewport.scrollTop;
    if (onnearend && viewport.scrollTop + viewport.clientHeight >= viewport.scrollHeight - rowHeight * 4) onnearend();
  }

  $effect(() => {
    if (!viewport) return;
    const ro = new ResizeObserver(() => { if (viewport) height = viewport.clientHeight; });
    ro.observe(viewport);
    height = viewport.clientHeight;
    return () => ro.disconnect();
  });

  export function scrollToIndex(index: number): void {
    if (!viewport) return;
    const top = index * rowHeight;
    const bottom = top + rowHeight;
    if (top < viewport.scrollTop) viewport.scrollTop = top;
    else if (bottom > viewport.scrollTop + viewport.clientHeight) viewport.scrollTop = bottom - viewport.clientHeight;
  }
  export function scrollToTop(): void { if (viewport) viewport.scrollTop = 0; }
</script>

<div class="viewport {cls}" bind:this={viewport} onscroll={onScroll}>
  <div class="spacer" style:height="{total}px">
    {#each visible as v (key ? key(v.item, v.index) : v.index)}
      <div class="row" style:transform="translateY({v.index * rowHeight}px)" style:height="{rowHeight}px">
        {@render row(v.item, v.index)}
      </div>
    {/each}
  </div>
  {#if footer}{@render footer()}{/if}
</div>

<style>
  .viewport { overflow-y: auto; overflow-x: hidden; height: 100%; position: relative; }
  .spacer { position: relative; width: 100%; }
  .row { position: absolute; top: 0; left: 0; right: 0; will-change: transform; }
</style>

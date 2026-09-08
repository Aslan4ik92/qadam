<script lang="ts">
  /** Vertical drag handle. Calls `onresize(deltaPx)` while dragging; `onend` when released. */
  interface Props { onresize: (dx: number) => void; onend?: () => void; label?: string }
  let { onresize, onend, label = 'Resize' }: Props = $props();
  let dragging = $state(false);
  let lastX = 0;

  function down(e: PointerEvent) {
    dragging = true;
    lastX = e.clientX;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    document.body.style.cursor = 'col-resize';
  }
  function move(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - lastX;
    lastX = e.clientX;
    if (dx) onresize(dx);
  }
  function up(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    document.body.style.cursor = '';
    onend?.();
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') { onresize(-16); onend?.(); e.preventDefault(); }
    if (e.key === 'ArrowRight') { onresize(16); onend?.(); e.preventDefault(); }
  }
</script>

<div class="splitter" class:dragging role="separator" aria-orientation="vertical" aria-label={label} tabindex="0"
  onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onkeydown={key}>
  <div class="bar"></div>
</div>

<style>
  .splitter { flex: 0 0 6px; width: 6px; cursor: col-resize; display: flex; justify-content: center; position: relative; z-index: 2; }
  .bar { width: 1px; height: 100%; background: var(--divider); transition: background var(--dur) var(--ease), width var(--dur) var(--ease); }
  .splitter:hover .bar, .splitter.dragging .bar, .splitter:focus-visible .bar { width: 3px; background: var(--accent); }
  .splitter:focus-visible { outline: none; }
</style>

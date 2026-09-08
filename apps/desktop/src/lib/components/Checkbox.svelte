<script lang="ts">
  import Icon from './Icon.svelte';
  import type { Snippet } from 'svelte';
  interface Props {
    checked: boolean;
    onchange?: (v: boolean) => void;
    label?: string;
    count?: number | string;
    disabled?: boolean;
    children?: Snippet;
    title?: string;
  }
  let { checked, onchange, label, count, disabled = false, children, title }: Props = $props();
</script>

<label class="cb" class:disabled {title}>
  <input type="checkbox" {checked} {disabled} onchange={(e) => onchange?.((e.currentTarget as HTMLInputElement).checked)} />
  <span class="box" aria-hidden="true">{#if checked}<Icon name="check" size={14} />{/if}</span>
  <span class="text ellipsis">{#if children}{@render children()}{:else}{label}{/if}</span>
  {#if count !== undefined}<span class="count tabular">{count}</span>{/if}
</label>

<style>
  .cb { display: flex; align-items: center; gap: 8px; min-height: 28px; padding: 0 6px; border-radius: var(--radius-sm); cursor: pointer; color: var(--fg); }
  .cb:hover { background: var(--surface-hover); }
  .cb.disabled { color: var(--fg-disabled); cursor: default; }
  .cb input { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .box {
    flex: 0 0 auto; width: 18px; height: 18px; border-radius: 4px; display: grid; place-items: center;
    border: 1px solid var(--stroke-control-strong); background: var(--surface); color: var(--accent-fg);
    transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
  }
  .cb input:checked + .box { background: var(--accent); border-color: var(--accent); }
  .cb input:focus-visible + .box { outline: 2px solid var(--focus); outline-offset: 1px; }
  .text { flex: 1 1 auto; min-width: 0; display: flex; align-items: center; gap: 6px; }
  .count { color: var(--fg-3); font-size: var(--fs-sm); }
</style>

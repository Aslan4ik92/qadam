<script lang="ts">
  interface Props {
    checked: boolean;
    onchange?: (v: boolean) => void;
    label?: string;
    hint?: string;
    disabled?: boolean;
    id?: string;
  }
  let { checked, onchange, label, hint, disabled = false, id }: Props = $props();
</script>

<label class="toggle" class:disabled>
  <span class="labels">
    {#if label}<span class="label">{label}</span>{/if}
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input {id} type="checkbox" role="switch" aria-checked={checked} {checked} {disabled} onchange={(e) => onchange?.((e.currentTarget as HTMLInputElement).checked)} />
  <span class="track" aria-hidden="true"><span class="knob"></span></span>
</label>

<style>
  .toggle { display: flex; align-items: center; gap: 12px; padding: 8px 0; cursor: pointer; }
  .toggle.disabled { color: var(--fg-disabled); cursor: default; }
  .labels { flex: 1 1 auto; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .label { font-size: var(--fs-md); }
  .hint { font-size: var(--fs-sm); color: var(--fg-2); line-height: 1.4; }
  input { position: absolute; opacity: 0; width: 1px; height: 1px; pointer-events: none; }
  .track {
    flex: 0 0 auto; width: 40px; height: 20px; border-radius: 10px; position: relative;
    border: 1px solid var(--stroke-control-strong); background: transparent;
    transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
  }
  .knob {
    position: absolute; top: 3px; left: 3px; width: 12px; height: 12px; border-radius: 50%;
    background: var(--fg-2); transition: transform var(--dur) var(--ease), background var(--dur) var(--ease), width var(--dur) var(--ease);
  }
  .toggle:hover .knob { width: 14px; }
  input:checked + .track { background: var(--accent); border-color: var(--accent); }
  input:checked + .track .knob { transform: translateX(20px); background: var(--accent-fg); }
  .toggle:hover input:checked + .track .knob { transform: translateX(18px); }
  input:focus-visible + .track { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>

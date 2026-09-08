/**
 * Small keyboard helpers. A "chord" is described as "Ctrl+Shift+C", "F3", "Escape", etc.
 * Matching is layout-independent for letters via `event.code` (KeyC) with a fallback to `key`,
 * so Ctrl+C works on Russian/Kazakh layouts too.
 */
export interface Chord {
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
}

export function parseChord(spec: string): Chord {
  const parts = spec.split('+').map((p) => p.trim());
  const key = parts.pop() ?? '';
  const mods = parts.map((p) => p.toLowerCase());
  return {
    key,
    ctrl: mods.includes('ctrl') || mods.includes('cmd') || mods.includes('mod'),
    shift: mods.includes('shift'),
    alt: mods.includes('alt')
  };
}

export function matchesChord(e: KeyboardEvent, spec: string): boolean {
  const c = parseChord(spec);
  const ctrl = e.ctrlKey || e.metaKey;
  if (c.ctrl !== ctrl || c.shift !== e.shiftKey || c.alt !== e.altKey) return false;
  const k = c.key;
  if (k.length === 1 && /[a-z]/i.test(k)) {
    return e.code === `Key${k.toUpperCase()}` || e.key.toLowerCase() === k.toLowerCase();
  }
  if (k === ',') return e.code === 'Comma' || e.key === ',';
  return e.key === k || e.code === k;
}

/** True when the event originates from a text-editing element (so single-key shortcuts should not fire). */
export function isEditableTarget(e: Event): boolean {
  const el = e.target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
}

/** Formats a chord for display: "Ctrl+Shift+C". */
export function displayChord(spec: string): string {
  return spec.replace(/\bmod\b/i, 'Ctrl');
}

import { describe, it, expect } from 'vitest';
import { filtersToRequest, defaultFilters, hasActiveFilters } from '../src/lib/stores/search.svelte';
import { highlightText, sanitizeSnippet } from '../src/lib/utils/highlight';
import { matchesChord, parseChord } from '../src/lib/utils/keys';

describe('filtersToRequest', () => {
  const now = new Date(2024, 2, 12, 15, 30);
  it('maps presets to unix bounds', () => {
    const today = filtersToRequest({ ...defaultFilters(), modified: 'today' }, now);
    expect(today.modifiedFrom).toBe(new Date(2024, 2, 12).getTime() / 1000);
    expect(today.modifiedTo).toBeNull();
    const week = filtersToRequest({ ...defaultFilters(), modified: 'week' }, now);
    expect(week.modifiedFrom).toBe(new Date(2024, 2, 12).getTime() / 1000 - 6 * 86400);
    const medium = filtersToRequest({ ...defaultFilters(), size: 'medium' }, now);
    expect(medium.sizeMin).toBe(100 * 1024);
    expect(medium.sizeMax).toBe(10 * 1024 * 1024);
  });
  it('maps custom ranges', () => {
    const r = filtersToRequest({ ...defaultFilters(), modified: 'custom', modifiedFrom: '2024-01-01', modifiedTo: '2024-01-31', size: 'custom', sizeMinKb: 10, sizeMaxKb: null }, now);
    expect(r.modifiedFrom).toBe(new Date(2024, 0, 1).getTime() / 1000);
    expect(r.modifiedTo).toBe(Math.floor(new Date(2024, 0, 31, 23, 59, 59).getTime() / 1000));
    expect(r.sizeMin).toBe(10240);
    expect(r.sizeMax).toBeNull();
  });
  it('detects active filters', () => {
    expect(hasActiveFilters(defaultFilters())).toBe(false);
    expect(hasActiveFilters({ ...defaultFilters(), withContentOnly: true })).toBe(true);
  });
});

describe('highlightText', () => {
  it('escapes and marks inflected forms', () => {
    const out = highlightText('Договор аренды <12>', 'договор аренда', 'smart');
    expect(out).toBe('<mark>Договор</mark> <mark>аренды</mark> &lt;12&gt;');
  });
  it('sanitizes stray tags but keeps mark', () => {
    expect(sanitizeSnippet('a <b>x</b> <mark>y</mark> <img src=x onerror=alert(1)>')).toBe('a x <mark>y</mark> ');
  });
});

describe('keys', () => {
  const ev = (init: Partial<KeyboardEvent>) => ({ ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, key: '', code: '', ...init }) as KeyboardEvent;
  it('parses chords', () => {
    expect(parseChord('Ctrl+Shift+C')).toEqual({ key: 'C', ctrl: true, shift: true, alt: false });
  });
  it('matches letters by physical key (works on Cyrillic layouts)', () => {
    expect(matchesChord(ev({ ctrlKey: true, key: 'с', code: 'KeyC' }), 'Ctrl+C')).toBe(true);
    expect(matchesChord(ev({ ctrlKey: true, key: 'c', code: 'KeyC' }), 'Ctrl+Shift+C')).toBe(false);
    expect(matchesChord(ev({ key: 'F3' }), 'F3')).toBe(true);
    expect(matchesChord(ev({ shiftKey: true, key: 'F3' }), 'F3')).toBe(false);
    expect(matchesChord(ev({ ctrlKey: true, key: ',', code: 'Comma' }), 'Ctrl+,')).toBe(true);
  });
});

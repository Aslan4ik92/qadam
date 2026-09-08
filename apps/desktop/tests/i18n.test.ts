import { describe, it, expect } from 'vitest';
import { ru } from '../src/lib/i18n/ru';
import { kk } from '../src/lib/i18n/kk';
import { en } from '../src/lib/i18n/en';
import { interpolate, pluralRu } from '../src/lib/i18n/i18n.svelte';

const dicts = { kk, en } as const;

describe('i18n completeness', () => {
  for (const [name, dict] of Object.entries(dicts)) {
    it(`${name} has every key of ru and nothing extra`, () => {
      const ruKeys = Object.keys(ru).sort();
      const keys = Object.keys(dict).sort();
      expect(keys).toEqual(ruKeys);
    });
    it(`${name} has no empty values`, () => {
      for (const [k, v] of Object.entries(dict)) expect(v.trim(), k).not.toBe('');
    });
    it(`${name} keeps the same placeholders as ru`, () => {
      for (const k of Object.keys(ru) as Array<keyof typeof ru>) {
        const ph = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort();
        expect(ph(dict[k]), k).toEqual(ph(ru[k]));
      }
    });
  }
  it('kk is actually Kazakh (contains Kazakh-specific letters)', () => {
    const text = Object.values(kk).join(' ');
    expect(/[әіңғүұқөһ]/i.test(text)).toBe(true);
    expect(kk['search.placeholder']).toBe('Файл мазмұны бойынша іздеу…');
  });
});

describe('interpolate', () => {
  it('replaces placeholders and leaves unknown ones', () => {
    expect(interpolate('Найдено {count} файлов за {ms} мс', { count: 128, ms: 12 })).toBe('Найдено 128 файлов за 12 мс');
    expect(interpolate('{a} {b}', { a: 1 })).toBe('1 {b}');
  });
});

describe('pluralRu', () => {
  it('categorises', () => {
    expect(pluralRu(1)).toBe('one');
    expect(pluralRu(21)).toBe('one');
    expect(pluralRu(2)).toBe('few');
    expect(pluralRu(11)).toBe('many');
    expect(pluralRu(128)).toBe('many');
    expect(pluralRu(0)).toBe('many');
  });
});

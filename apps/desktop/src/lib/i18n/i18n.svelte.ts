import { ru, type Dictionary, type I18nKey } from './ru';
import { kk } from './kk';
import { en } from './en';
import type { UiLanguage } from '../api/types';

export const dictionaries: Record<UiLanguage, Dictionary> = { ru, kk, en };
export const LOCALES: Record<UiLanguage, string> = { ru: 'ru-RU', kk: 'kk-KZ', en: 'en-US' };
export const LANGUAGE_NAMES: Record<UiLanguage, string> = { ru: 'Русский', kk: 'Қазақша', en: 'English' };

export type Params = Record<string, string | number>;

/** Replaces {name} placeholders. Exported separately so it can be unit-tested without Svelte. */
export function interpolate(template: string, params?: Params): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (m, k: string) => (k in params ? String(params[k]) : m));
}

/** Russian-style plural category: one / few / many. Used to pick a template variant. */
export function pluralRu(n: number): 'one' | 'few' | 'many' {
  const a = Math.abs(n) % 100;
  const b = a % 10;
  if (a > 10 && a < 20) return 'many';
  if (b > 1 && b < 5) return 'few';
  if (b === 1) return 'one';
  return 'many';
}

class I18nState {
  lang = $state<UiLanguage>('ru');
  get locale(): string { return LOCALES[this.lang]; }
  get dict(): Dictionary { return dictionaries[this.lang]; }
}

export const i18n = new I18nState();

export function setLanguage(lang: UiLanguage): void {
  i18n.lang = lang;
  try { document.documentElement.lang = lang; } catch { /* SSR / tests */ }
}

export function t(key: I18nKey, params?: Params): string {
  const template = i18n.dict[key] ?? ru[key] ?? key;
  return interpolate(template, params);
}

/** "Found N files" with Russian plural agreement; other languages use the base form. */
export function tFound(count: number, ms: number): string {
  let key: I18nKey = 'search.found';
  if (i18n.lang === 'ru') {
    const p = pluralRu(count);
    key = p === 'one' ? 'search.found.one' : p === 'few' ? 'search.found.few' : 'search.found';
  } else if (count === 1) {
    key = 'search.found.one';
  }
  return t(key, { count: new Intl.NumberFormat(i18n.locale).format(count), ms });
}

export type { I18nKey, Dictionary };

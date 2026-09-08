import type { UiLanguage } from '../api/types';

const LOCALES: Record<UiLanguage, string> = { ru: 'ru-RU', kk: 'kk-KZ', en: 'en-US' };
const UNITS: Record<UiLanguage, string[]> = {
  ru: ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'],
  kk: ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'],
  en: ['B', 'KB', 'MB', 'GB', 'TB']
};

/**
 * Resolves the Intl locale for a UI language. Kazakh falls back to ru-RU when the runtime's ICU lacks kk data
 * (identical numeric formats: dd.mm.yyyy, space grouping, comma decimal) instead of the browser default.
 */
export function localeOf(lang: UiLanguage): string {
  const loc = LOCALES[lang];
  if (lang !== 'kk') return loc;
  try { return Intl.DateTimeFormat.supportedLocalesOf([loc]).length ? loc : 'ru-RU'; } catch { return 'ru-RU'; }
}

/** "1,2 МБ" (ru), "1.2 MB" (en). Whole numbers below 1 KB show no fraction. */
export function formatSize(bytes: number, lang: UiLanguage = 'ru'): string {
  const units = UNITS[lang];
  let v = Math.max(0, bytes);
  let i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  const digits = i === 0 ? 0 : v >= 100 ? 0 : 1;
  const num = new Intl.NumberFormat(localeOf(lang), { minimumFractionDigits: 0, maximumFractionDigits: digits }).format(v);
  return `${num}\u00a0${units[i]}`;
}

export function formatNumber(n: number, lang: UiLanguage = 'ru'): string {
  return new Intl.NumberFormat(localeOf(lang)).format(n);
}

/** Short numeric date: ru/kk "12.03.2024", en "03/12/2024". */
export function formatDate(unixSeconds: number, lang: UiLanguage = 'ru'): string {
  if (!unixSeconds) return '—';
  return new Intl.DateTimeFormat(localeOf(lang), { day: '2-digit', month: '2-digit', year: 'numeric' }).format(new Date(unixSeconds * 1000));
}

export function formatDateTime(unixSeconds: number, lang: UiLanguage = 'ru'): string {
  if (!unixSeconds) return '—';
  return new Intl.DateTimeFormat(localeOf(lang), {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit'
  }).format(new Date(unixSeconds * 1000));
}

/** Relative time like "5 мин назад" / "5 min ago" via Intl.RelativeTimeFormat. */
export function formatRelative(unixSeconds: number, lang: UiLanguage = 'ru', nowMs: number = Date.now(), justNow = 'только что'): string {
  const diff = Math.round(nowMs / 1000 - unixSeconds); // seconds ago
  if (diff < 45) return justNow;
  const abs = Math.abs(diff);
  const unit: Intl.RelativeTimeFormatUnit = abs < 3600 ? 'minute' : abs < 86400 ? 'hour' : abs < 86400 * 30 ? 'day' : abs < 86400 * 365 ? 'month' : 'year';
  const div: Record<string, number> = { minute: 60, hour: 3600, day: 86400, month: 86400 * 30, year: 86400 * 365 };
  const n = Math.round(diff / div[unit]);
  const supported = Intl.RelativeTimeFormat.supportedLocalesOf([LOCALES[lang]]).length > 0;
  if (!supported && lang === 'kk') {
    // Some WebView/ICU builds ship without Kazakh relative-time data; fall back to hand-written forms.
    const KK: Record<string, string> = { minute: 'мин', hour: 'сағ', day: 'күн', month: 'ай', year: 'жыл' };
    return `${n} ${KK[unit]} бұрын`;
  }
  const rtf = new Intl.RelativeTimeFormat(LOCALES[lang], { numeric: 'always', style: 'short' });
  return rtf.format(-n, unit);
}

/** Formats a duration in ms as "1:05" or "12 с". */
export function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rest = s % 60;
  return `${m}:${rest.toString().padStart(2, '0')}`;
}

/** Converts a `<input type="date">` value (YYYY-MM-DD) to unix seconds at local midnight; `endOfDay` adds 23:59:59. */
export function dateInputToUnix(value: string, endOfDay = false): number | null {
  if (!value) return null;
  const [y, m, d] = value.split('-').map(Number);
  if (!y || !m || !d) return null;
  const date = new Date(y, m - 1, d, endOfDay ? 23 : 0, endOfDay ? 59 : 0, endOfDay ? 59 : 0);
  return Math.floor(date.getTime() / 1000);
}

export function unixToDateInput(unix: number | null | undefined): string {
  if (!unix) return '';
  const d = new Date(unix * 1000);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

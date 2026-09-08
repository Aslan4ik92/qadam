import type { UiLanguage } from '../api/types';

const LOCALES: Record<UiLanguage, string> = { ru: 'ru-RU', kk: 'kk-KZ', en: 'en-US' };
const UNITS: Record<UiLanguage, string[]> = {
  ru: ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'],
  kk: ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'],
  en: ['B', 'KB', 'MB', 'GB', 'TB']
};

export function localeOf(lang: UiLanguage): string { return LOCALES[lang]; }

/** "1,2 МБ" (ru), "1.2 MB" (en). Whole numbers below 1 KB show no fraction. */
export function formatSize(bytes: number, lang: UiLanguage = 'ru'): string {
  const units = UNITS[lang];
  let v = Math.max(0, bytes);
  let i = 0;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  const digits = i === 0 ? 0 : v >= 100 ? 0 : 1;
  const num = new Intl.NumberFormat(LOCALES[lang], { minimumFractionDigits: 0, maximumFractionDigits: digits }).format(v);
  return `${num}\u00a0${units[i]}`;
}

export function formatNumber(n: number, lang: UiLanguage = 'ru'): string {
  return new Intl.NumberFormat(LOCALES[lang]).format(n);
}

/** Short numeric date: ru/kk "12.03.2024", en "03/12/2024". */
export function formatDate(unixSeconds: number, lang: UiLanguage = 'ru'): string {
  if (!unixSeconds) return '—';
  return new Intl.DateTimeFormat(LOCALES[lang], { day: '2-digit', month: '2-digit', year: 'numeric' }).format(new Date(unixSeconds * 1000));
}

export function formatDateTime(unixSeconds: number, lang: UiLanguage = 'ru'): string {
  if (!unixSeconds) return '—';
  return new Intl.DateTimeFormat(LOCALES[lang], {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit'
  }).format(new Date(unixSeconds * 1000));
}

/** Relative time like "5 мин назад" / "5 min ago" via Intl.RelativeTimeFormat. */
export function formatRelative(unixSeconds: number, lang: UiLanguage = 'ru', nowMs: number = Date.now(), justNow = 'только что'): string {
  const diff = Math.round(nowMs / 1000 - unixSeconds); // seconds ago
  if (diff < 45) return justNow;
  const rtf = new Intl.RelativeTimeFormat(LOCALES[lang], { numeric: 'always', style: 'short' });
  const abs = Math.abs(diff);
  if (abs < 3600) return rtf.format(-Math.round(diff / 60), 'minute');
  if (abs < 86400) return rtf.format(-Math.round(diff / 3600), 'hour');
  if (abs < 86400 * 30) return rtf.format(-Math.round(diff / 86400), 'day');
  if (abs < 86400 * 365) return rtf.format(-Math.round(diff / (86400 * 30)), 'month');
  return rtf.format(-Math.round(diff / (86400 * 365)), 'year');
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

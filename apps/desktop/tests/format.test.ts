import { describe, it, expect } from 'vitest';
import { formatSize, formatDate, formatRelative, dateInputToUnix, unixToDateInput } from '../src/lib/utils/format';
import { ellipsizePath, baseName, dirName, extension, isUnder } from '../src/lib/utils/path';

describe('formatSize', () => {
  it('formats Russian units with comma decimal', () => {
    expect(formatSize(1258291, 'ru')).toBe('1,2\u00a0МБ');
    expect(formatSize(512, 'ru')).toBe('512\u00a0Б');
    expect(formatSize(2048, 'ru')).toBe('2\u00a0КБ');
  });
  it('formats Kazakh units', () => {
    expect(formatSize(3 * 1024 * 1024 * 1024, 'kk')).toBe('3\u00a0ГБ');
  });
  it('formats English units', () => {
    expect(formatSize(1258291, 'en')).toBe('1.2\u00a0MB');
    expect(formatSize(150 * 1024, 'en')).toBe('150\u00a0KB');
  });
});

describe('formatDate', () => {
  const ts = Math.floor(new Date(2024, 2, 12, 12).getTime() / 1000); // 12 March 2024 local
  it('uses dd.mm.yyyy for ru and kk', () => {
    expect(formatDate(ts, 'ru')).toBe('12.03.2024');
    expect(formatDate(ts, 'kk')).toBe('12.03.2024');
  });
  it('uses mm/dd/yyyy for en', () => {
    expect(formatDate(ts, 'en')).toBe('03/12/2024');
  });
});

describe('formatRelative', () => {
  const now = Date.now();
  it('returns "just now" for very recent timestamps', () => {
    expect(formatRelative(Math.floor(now / 1000) - 10, 'ru', now, 'только что')).toBe('только что');
  });
  it('returns minutes ago', () => {
    expect(formatRelative(Math.floor(now / 1000) - 5 * 60, 'en', now)).toMatch(/5 min\.? ago/);
    expect(formatRelative(Math.floor(now / 1000) - 5 * 60, 'ru', now)).toMatch(/5 мин/);
  });
});

describe('date inputs', () => {
  it('round-trips', () => {
    const unix = dateInputToUnix('2024-03-12');
    expect(unix).not.toBeNull();
    expect(unixToDateInput(unix)).toBe('2024-03-12');
    expect(dateInputToUnix('2024-03-12', true)! - unix!).toBe(86399);
    expect(dateInputToUnix('')).toBeNull();
  });
});

describe('path utils', () => {
  const p = 'D:\\Документы\\Договоры\\Аренда\\2024\\Договор аренды №12.docx';
  it('splits names and dirs', () => {
    expect(baseName(p)).toBe('Договор аренды №12.docx');
    expect(dirName(p)).toBe('D:\\Документы\\Договоры\\Аренда\\2024');
    expect(dirName('C:\\file.txt')).toBe('C:\\');
    expect(extension(p)).toBe('docx');
  });
  it('middle-ellipsizes long paths keeping drive and tail', () => {
    const e = ellipsizePath(p, 40);
    expect(e.startsWith('D:\\…\\')).toBe(true);
    expect(e.endsWith('Договор аренды №12.docx')).toBe(true);
    expect(e.length).toBeLessThanOrEqual(45);
    expect(ellipsizePath('C:\\short.txt', 40)).toBe('C:\\short.txt');
  });
  it('checks containment case-insensitively', () => {
    expect(isUnder(p, 'd:\\документы')).toBe(true);
    expect(isUnder(p, 'D:\\Документы2')).toBe(false);
  });
});

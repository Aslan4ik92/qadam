import { describe, it, expect } from 'vitest';
import { SAMPLE_DOCS, mockSearch, queryTerms, stem, buildPreview, interpret } from '../src/lib/api/mock';
import type { SearchRequest } from '../src/lib/api/types';

const base: SearchRequest = {
  query: '', mode: 'smart', roots: [], categories: [], extensions: [], withContentOnly: false,
  sort: 'relevance', offset: 0, limit: 50, snippetChars: 160
};

describe('queryTerms', () => {
  it('drops operators and field prefixes', () => {
    expect(queryTerms('договор OR шарт -черновик name:отчёт ext:xlsx аренд* слово~', 'smart'))
      .toEqual(['договор', 'шарт', 'отчёт', 'xlsx', 'аренд', 'слово']);
  });
  it('keeps quoted phrases whole in exact mode', () => {
    expect(queryTerms('"договор аренды" 2024', 'exact')).toEqual(['договор аренды', '2024']);
  });
  it('stems long words', () => {
    expect(stem('аренды')).toBe('аренд');
    expect(stem('договор')).toBe('догов');
    expect(stem('pdf')).toBe('pdf');
  });
});

describe('mockSearch', () => {
  it('has a realistic multilingual sample', () => {
    expect(SAMPLE_DOCS.length).toBeGreaterThanOrEqual(40);
    expect(SAMPLE_DOCS.some((d) => d.path.startsWith('D:\\Жұмыс'))).toBe(true);
    expect(SAMPLE_DOCS.some((d) => d.path.startsWith('C:\\Users\\Aslan'))).toBe(true);
  });
  it('matches inflected forms in smart mode and marks snippets', () => {
    const r = mockSearch(SAMPLE_DOCS, { ...base, query: 'договор аренды' });
    expect(r.total).toBeGreaterThan(3);
    expect(r.hits[0].snippet).toContain('<mark>');
    expect(r.hits.every((h) => /догов/i.test(h.path + h.snippet))).toBe(true);
    expect(r.interpretation).toBe('догов* AND аренд*');
    expect(r.facets.categories.length).toBeGreaterThan(1);
  });
  it('filters by category, extension and root', () => {
    const all = mockSearch(SAMPLE_DOCS, { ...base, query: 'аренд' });
    const sheets = mockSearch(SAMPLE_DOCS, { ...base, query: 'аренд', categories: ['spreadsheet'] });
    expect(sheets.total).toBeLessThan(all.total);
    expect(sheets.hits.every((h) => h.category === 'spreadsheet')).toBe(true);
    const xlsx = mockSearch(SAMPLE_DOCS, { ...base, query: 'аренд', extensions: ['xlsx'] });
    expect(xlsx.hits.every((h) => h.ext === 'xlsx')).toBe(true);
    const kkRoot = mockSearch(SAMPLE_DOCS, { ...base, roots: ['D:\\Жұмыс'] });
    expect(kkRoot.hits.every((h) => h.root === 'D:\\Жұмыс')).toBe(true);
    expect(kkRoot.total).toBeGreaterThan(5);
  });
  it('sorts and paginates', () => {
    const bySize = mockSearch(SAMPLE_DOCS, { ...base, sort: 'size_desc', limit: 5 });
    expect(bySize.hits).toHaveLength(5);
    for (let i = 1; i < bySize.hits.length; i++) expect(bySize.hits[i - 1].size).toBeGreaterThanOrEqual(bySize.hits[i].size);
    const page2 = mockSearch(SAMPLE_DOCS, { ...base, sort: 'name_asc', offset: 5, limit: 5 });
    const page1 = mockSearch(SAMPLE_DOCS, { ...base, sort: 'name_asc', offset: 0, limit: 5 });
    expect(page1.hits.map((h) => h.path)).not.toEqual(page2.hits.map((h) => h.path));
    expect(page1.total).toBe(SAMPLE_DOCS.length);
  });
  it('honours withContentOnly and size bounds', () => {
    const r = mockSearch(SAMPLE_DOCS, { ...base, withContentOnly: true });
    expect(r.hits.every((h) => h.hasContent)).toBe(true);
    const big = mockSearch(SAMPLE_DOCS, { ...base, sizeMin: 10 * 1024 * 1024 });
    expect(big.hits.every((h) => h.size >= 10 * 1024 * 1024)).toBe(true);
  });
  it('escapes HTML in snippets', () => {
    const r = mockSearch(SAMPLE_DOCS, { ...base, query: 'title' });
    const html = r.hits.find((h) => h.name === 'index.html');
    expect(html).toBeDefined();
    expect(html!.snippet).toContain('&lt;');
    expect(html!.snippet.replace(/<\/?mark>/g, '')).not.toMatch(/<[^&]/);
  });
});

describe('buildPreview / interpret', () => {
  it('splits into highlighted segments', () => {
    const doc = SAMPLE_DOCS[0];
    const p = buildPreview(doc, 'аренды', 'smart');
    expect(p.totalMatches).toBeGreaterThan(3);
    expect(p.segments.filter((s) => s.highlight)).toHaveLength(p.totalMatches);
    expect(p.segments.map((s) => s.text).join('')).toBe(doc.content);
  });
  it('interprets exact mode as quoted terms', () => {
    expect(interpret('"договор аренды"', 'exact')).toBe('"договор аренды"');
    expect(interpret('', 'smart')).toBe('');
  });
});

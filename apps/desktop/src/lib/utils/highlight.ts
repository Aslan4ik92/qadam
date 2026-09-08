import { queryTerms, stem } from './query';
import type { SearchMode } from '../api/types';

export { escapeHtml } from './query';
import { escapeHtml } from './query';

/**
 * Wraps query words found in `text` with <mark>. Escapes HTML first, so the output is safe for {@html}.
 * In smart mode words are matched by a short prefix so inflected forms light up too.
 */
export function highlightText(text: string, query: string, mode: SearchMode): string {
  const escaped = escapeHtml(text);
  const terms = queryTerms(query, mode).filter((t) => t.length >= 2);
  if (terms.length === 0) return escaped;
  const bases = [...new Set(terms.map((t) => (mode === 'smart' ? stem(t) : t)))]
    .sort((a, b) => b.length - a.length)
    .map((b) => b.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  const re = new RegExp(`(${bases.join('|')})${mode === 'smart' ? '[\\p{L}\\p{N}]{0,4}' : ''}`, 'giu');
  return escaped.replace(re, (m) => `<mark>${m}</mark>`);
}

/** Strips everything except <mark> tags from backend-provided snippet HTML (defence in depth). */
export function sanitizeSnippet(html: string): string {
  return html.replace(/<(?!\/?mark\b)[^>]*>/gi, '');
}

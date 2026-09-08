import type { SearchMode } from '../api/types';

export const DEFAULT_EXCLUDE_GLOBS = [
  '**/node_modules/**', '**/.git/**', '**/target/**', '**/dist/**', '**/$RECYCLE.BIN/**',
  '**/System Volume Information/**', '**/AppData/Local/Temp/**', '**/*.tmp', '**/~$*'
];

export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

/** Splits a query into plain terms, dropping operators (OR/AND/NOT, -word) and field prefixes (name:, ext:, …). */
export function queryTerms(query: string, mode: SearchMode): string[] {
  const terms: string[] = [];
  const re = /"([^"]+)"|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(query)) !== null) {
    if (m[1] !== undefined) {
      if (mode === 'exact') terms.push(m[1].toLowerCase());
      else terms.push(...m[1].toLowerCase().split(/\s+/).filter(Boolean));
      continue;
    }
    let t = m[2];
    if (/^(or|and|not)$/i.test(t)) continue;
    if (t.startsWith('-')) continue;
    const colon = t.indexOf(':');
    if (colon > 0 && /^(name|ext|path|content)$/i.test(t.slice(0, colon))) t = t.slice(colon + 1);
    t = t.replace(/[*~]+$/g, '').toLowerCase();
    if (t) terms.push(t);
  }
  return terms;
}

/** Crude stemming used for client-side highlighting and the mock's "smart" mode: strips a short inflectional tail. */
export function stem(term: string): string {
  if (term.length <= 4) return term;
  if (term.length <= 6) return term.slice(0, -1);
  return term.slice(0, -2);
}

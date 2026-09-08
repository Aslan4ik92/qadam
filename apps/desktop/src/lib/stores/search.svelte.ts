import * as api from '../api/backend';
import { errorMessage } from '../api/backend';
import type { Facets, FileCategory, Preview, SearchHit, SearchMode, SearchRequest, SortOrder } from '../api/types';
import { debounce } from '../utils/debounce';
import { dateInputToUnix } from '../utils/format';
import { settings } from './settings.svelte';
import { ui } from './ui.svelte';

export type ModifiedPreset = 'any' | 'today' | 'week' | 'month' | 'year' | 'custom';
export type SizePreset = 'any' | 'small' | 'medium' | 'large' | 'custom';

const SORTS: SortOrder[] = ['relevance', 'modified_desc', 'modified_asc', 'size_desc', 'size_asc', 'name_asc', 'name_desc'];
export const SORT_ORDERS = SORTS;
const emptyFacets = (): Facets => ({ categories: [], extensions: [], roots: [] });

export interface Filters {
  roots: string[];
  categories: FileCategory[];
  extensions: string[];
  modified: ModifiedPreset;
  modifiedFrom: string; // yyyy-mm-dd
  modifiedTo: string;
  size: SizePreset;
  sizeMinKb: number | null;
  sizeMaxKb: number | null;
  withContentOnly: boolean;
}

export function defaultFilters(): Filters {
  return {
    roots: [], categories: [], extensions: [], modified: 'any', modifiedFrom: '', modifiedTo: '',
    size: 'any', sizeMinKb: null, sizeMaxKb: null, withContentOnly: false
  };
}

/** Converts the UI filter state into request fields. Pure, unit-tested. */
export function filtersToRequest(f: Filters, now: Date = new Date()): Pick<SearchRequest, 'roots' | 'categories' | 'extensions' | 'sizeMin' | 'sizeMax' | 'modifiedFrom' | 'modifiedTo' | 'withContentOnly'> {
  let modifiedFrom: number | null = null;
  let modifiedTo: number | null = null;
  const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() / 1000;
  switch (f.modified) {
    case 'today': modifiedFrom = startOfDay; break;
    case 'week': modifiedFrom = startOfDay - 6 * 86400; break;
    case 'month': modifiedFrom = startOfDay - 29 * 86400; break;
    case 'year': modifiedFrom = startOfDay - 364 * 86400; break;
    case 'custom':
      modifiedFrom = dateInputToUnix(f.modifiedFrom);
      modifiedTo = dateInputToUnix(f.modifiedTo, true);
      break;
    default: break;
  }
  let sizeMin: number | null = null;
  let sizeMax: number | null = null;
  const KB = 1024, MB = 1024 * 1024;
  switch (f.size) {
    case 'small': sizeMax = 100 * KB; break;
    case 'medium': sizeMin = 100 * KB; sizeMax = 10 * MB; break;
    case 'large': sizeMin = 10 * MB; break;
    case 'custom':
      sizeMin = f.sizeMinKb != null ? f.sizeMinKb * KB : null;
      sizeMax = f.sizeMaxKb != null ? f.sizeMaxKb * KB : null;
      break;
    default: break;
  }
  return {
    roots: f.roots, categories: f.categories, extensions: f.extensions,
    sizeMin, sizeMax, modifiedFrom, modifiedTo, withContentOnly: f.withContentOnly
  };
}

export function hasActiveFilters(f: Filters): boolean {
  return f.roots.length > 0 || f.categories.length > 0 || f.extensions.length > 0 || f.modified !== 'any' || f.size !== 'any' || f.withContentOnly;
}

class SearchState {
  query = $state('');
  mode = $state<SearchMode>('smart');
  sort = $state<SortOrder>(SORTS.includes(ui.persistedSort as SortOrder) ? (ui.persistedSort as SortOrder) : 'relevance');
  filters = $state<Filters>(defaultFilters());

  hits = $state<SearchHit[]>([]);
  total = $state(0);
  facets = $state<Facets>(emptyFacets());
  tookMs = $state(0);
  interpretation = $state('');
  loading = $state(false);
  loadingMore = $state(false);
  error = $state<string | null>(null);
  /** True once at least one search has completed since the query/filters last changed. */
  searched = $state(false);
  selectedIndex = $state(-1);

  preview = $state<Preview | null>(null);
  previewLoading = $state(false);
  previewError = $state<string | null>(null);
  /** 0-based index of the current match in the preview. */
  currentMatch = $state(0);

  #seq = 0;
  #previewSeq = 0;

  get selected(): SearchHit | null { return this.selectedIndex >= 0 ? this.hits[this.selectedIndex] ?? null : null; }
  get hasMore(): boolean { return this.hits.length < this.total; }
  get active(): boolean { return this.query.trim().length > 0 || hasActiveFilters(this.filters); }

  #debouncedRun = debounce(() => { void this.run(); }, 250);
  #debouncedPreview = debounce(() => { void this.loadPreview(); }, 120);

  buildRequest(offset: number): SearchRequest {
    const limit = Math.max(10, settings.value.pageSize || 50);
    return {
      query: this.query.trim(), mode: this.mode, ...filtersToRequest(this.filters),
      sort: this.sort, offset, limit, snippetChars: 180
    };
  }

  /** Called on input changes: debounced search. */
  schedule(): void { this.#debouncedRun(); }
  /** Immediate search (Enter, filter change). */
  runNow(): void { this.#debouncedRun.cancel(); void this.run(); }

  setQuery(q: string): void { this.query = q; this.schedule(); }
  setMode(m: SearchMode): void { if (m !== this.mode) { this.mode = m; this.runNow(); } }
  setSort(s: SortOrder): void { if (s !== this.sort) { this.sort = s; ui.persist(s); this.runNow(); } }
  setFilters(patch: Partial<Filters>): void { this.filters = { ...this.filters, ...patch }; this.runNow(); }
  resetFilters(): void { this.filters = defaultFilters(); this.runNow(); }
  toggleIn<T extends string>(list: T[], value: T): T[] { return list.includes(value) ? list.filter((x) => x !== value) : [...list, value]; }

  clear(): void {
    this.#debouncedRun.cancel();
    this.query = '';
    this.#seq++;
    this.hits = []; this.total = 0; this.tookMs = 0; this.interpretation = ''; this.error = null;
    this.loading = false; this.searched = false; this.selectedIndex = -1;
    this.preview = null; this.previewError = null;
    if (hasActiveFilters(this.filters)) void this.run();
  }

  async run(): Promise<void> {
    const seq = ++this.#seq;
    if (!this.active) {
      this.hits = []; this.total = 0; this.facets = emptyFacets(); this.tookMs = 0; this.interpretation = '';
      this.error = null; this.loading = false; this.searched = false; this.selectedIndex = -1; this.preview = null;
      return;
    }
    this.loading = true;
    this.error = null;
    try {
      const res = await api.search(this.buildRequest(0));
      if (seq !== this.#seq) return;
      const prevPath = this.selected?.path ?? null;
      this.hits = res.hits; this.total = res.total; this.facets = res.facets; this.tookMs = res.tookMs;
      this.interpretation = res.interpretation;
      this.searched = true;
      const keep = prevPath ? res.hits.findIndex((h) => h.path === prevPath) : -1;
      this.select(keep >= 0 ? keep : res.hits.length > 0 ? 0 : -1);
    } catch (e) {
      if (seq !== this.#seq) return;
      this.error = errorMessage(e);
      this.hits = []; this.total = 0; this.searched = true; this.selectedIndex = -1;
    } finally {
      if (seq === this.#seq) this.loading = false;
    }
  }

  async loadMore(): Promise<void> {
    if (this.loadingMore || this.loading || !this.hasMore) return;
    const seq = this.#seq;
    this.loadingMore = true;
    try {
      const res = await api.search(this.buildRequest(this.hits.length));
      if (seq !== this.#seq) return;
      const seen = new Set(this.hits.map((h) => h.path));
      this.hits = [...this.hits, ...res.hits.filter((h) => !seen.has(h.path))];
      this.total = res.total;
    } catch (e) {
      ui.error(errorMessage(e));
    } finally {
      this.loadingMore = false;
    }
  }

  select(index: number): void {
    if (index < -1 || index >= this.hits.length) return;
    if (index === this.selectedIndex && (this.preview?.path === this.hits[index]?.path)) return;
    this.selectedIndex = index;
    this.currentMatch = 0;
    if (index < 0) { this.preview = null; this.previewError = null; this.#debouncedPreview.cancel(); return; }
    this.previewError = null;
    this.#debouncedPreview();
  }

  moveSelection(delta: number): void {
    if (this.hits.length === 0) return;
    const next = Math.min(this.hits.length - 1, Math.max(0, (this.selectedIndex < 0 ? 0 : this.selectedIndex + delta)));
    this.select(next);
    if (next >= this.hits.length - 5 && this.hasMore) void this.loadMore();
  }

  async loadPreview(): Promise<void> {
    const hit = this.selected;
    if (!hit) return;
    const seq = ++this.#previewSeq;
    this.previewLoading = true;
    try {
      const p = await api.getPreview(hit.path, this.query.trim(), this.mode);
      if (seq !== this.#previewSeq) return;
      this.preview = p;
      this.currentMatch = 0;
      this.previewError = null;
    } catch (e) {
      if (seq !== this.#previewSeq) return;
      this.preview = null;
      this.previewError = errorMessage(e);
    } finally {
      if (seq === this.#previewSeq) this.previewLoading = false;
    }
  }

  nextMatch(delta: 1 | -1): void {
    const n = this.preview?.totalMatches ?? 0;
    if (n === 0) return;
    this.currentMatch = (this.currentMatch + delta + n) % n;
  }
}

export const search = new SearchState();

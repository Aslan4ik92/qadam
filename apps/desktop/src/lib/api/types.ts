export type UiLanguage = 'ru' | 'kk' | 'en';
export type Theme = 'system' | 'light' | 'dark';
export interface IndexRoot { path: string; enabled: boolean }
export interface Settings {
  version: number; roots: IndexRoot[]; excludeGlobs: string[]; includeExtensions: string[];
  excludeExtensions: string[]; maxFileSizeBytes: number; storeContent: boolean; indexHidden: boolean;
  followLinks: boolean; watchChanges: boolean; workerThreads: number; writerMemoryMb: number;
  pageSize: number; uiLanguage: UiLanguage; theme: Theme; reindexOnStart: boolean; extractPdf: boolean;
}
export type FileCategory = 'document'|'spreadsheet'|'presentation'|'pdf'|'text'|'code'|'ebook'|'web'|'data'|'email'|'other';
export type SearchMode = 'smart' | 'exact';
export type SortOrder = 'relevance'|'modified_desc'|'modified_asc'|'size_desc'|'size_asc'|'name_asc'|'name_desc';
export interface SearchRequest {
  query: string; mode: SearchMode; roots: string[]; categories: FileCategory[]; extensions: string[];
  sizeMin?: number | null; sizeMax?: number | null; modifiedFrom?: number | null; modifiedTo?: number | null; // unix seconds
  withContentOnly: boolean; sort: SortOrder; offset: number; limit: number; snippetChars: number;
}
export interface SearchHit {
  path: string; name: string; ext: string; category: FileCategory; root: string; size: number;
  modified: number; /* unix seconds */ score: number; hasContent: boolean;
  snippet: string; /* HTML-escaped text with <mark>…</mark> around matches — safe to render with {@html} */
  encoding: string | null;
}
export interface FacetCount { value: string; count: number }
export interface Facets { categories: FacetCount[]; extensions: FacetCount[]; roots: FacetCount[] }
export interface SearchResponse { total: number; hits: SearchHit[]; facets: Facets; tookMs: number; interpretation: string; offset: number; limit: number }
export type IndexPhase = 'idle'|'scanning'|'extracting'|'cleaning'|'committing'|'done'|'cancelled'|'failed';
export interface IndexProgress {
  running: boolean; phase: IndexPhase; scanned: number; queued: number; indexed: number; unchanged: number;
  nameOnly: number; failed: number; deleted: number; bytes: number; current: string | null; error: string | null; elapsedMs: number;
}
export interface RootStats { path: string; enabled: boolean; exists: boolean; documents: number }
export interface IndexStats {
  documents: number; withContent: number; indexSizeBytes: number; lastIndexed: number | null; roots: RootStats[];
  indexing: boolean; watching: boolean; schemaVersion: number; dataDir: string;
}
export interface PreviewSegment { text: string; highlight: boolean }
export interface Preview { path: string; segments: PreviewSegment[]; totalMatches: number; truncated: boolean; source: 'index' | 'file'; encoding: string | null; chars: number }
export interface DriveInfo { path: string; label: string }
export interface AppInfo { version: string; dataDir: string; logDir: string; platform: string }

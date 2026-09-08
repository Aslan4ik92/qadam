export interface Debounced<A extends unknown[]> {
  (...args: A): void;
  /** Runs the pending call right now (if any). */
  flush(): void;
  cancel(): void;
  readonly pending: boolean;
}

export function debounce<A extends unknown[]>(fn: (...args: A) => void, wait: number): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let lastArgs: A | null = null;
  const run = () => {
    timer = null;
    if (lastArgs) { const a = lastArgs; lastArgs = null; fn(...a); }
  };
  const debounced = ((...args: A) => {
    lastArgs = args;
    if (timer) clearTimeout(timer);
    timer = setTimeout(run, wait);
  }) as Debounced<A>;
  debounced.flush = () => { if (timer) { clearTimeout(timer); run(); } };
  debounced.cancel = () => { if (timer) clearTimeout(timer); timer = null; lastArgs = null; };
  Object.defineProperty(debounced, 'pending', { get: () => timer !== null });
  return debounced;
}

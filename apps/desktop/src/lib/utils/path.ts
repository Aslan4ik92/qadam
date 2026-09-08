/** Returns the file name portion of a Windows or POSIX path. */
export function baseName(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  return i >= 0 ? path.slice(i + 1) : path;
}

/** Returns the parent directory (without a trailing separator, except for drive roots like "C:\"). */
export function dirName(path: string): string {
  const i = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  if (i < 0) return '';
  const dir = path.slice(0, i);
  return /^[A-Za-z]:$/.test(dir) ? dir + '\\' : dir;
}

export function extension(name: string): string {
  const base = baseName(name);
  const i = base.lastIndexOf('.');
  return i > 0 ? base.slice(i + 1).toLowerCase() : '';
}

/** Path segments (drive or root kept as the first segment). */
export function splitPath(path: string): string[] {
  return path.split(/[\\/]+/).filter(Boolean);
}

/**
 * Middle-ellipsizes a path so it fits roughly `max` characters, keeping the drive and the
 * last segments intact: "D:\Документы\…\Договоры\Договор.docx".
 */
export function ellipsizePath(path: string, max = 60): string {
  if (path.length <= max) return path;
  const sep = path.includes('\\') ? '\\' : '/';
  const parts = splitPath(path);
  if (parts.length <= 2) return path.slice(0, Math.max(1, max - 1)) + '…';
  const head = parts[0];
  const tail: string[] = [parts[parts.length - 1]];
  let len = head.length + 2 + tail[0].length; // head + sep + … + sep + tail
  for (let i = parts.length - 2; i >= 1; i--) {
    const candidate = parts[i].length + 1;
    if (len + candidate > max) break;
    tail.unshift(parts[i]);
    len += candidate;
  }
  if (tail.length === parts.length - 1) return path;
  return `${head}${sep}…${sep}${tail.join(sep)}`;
}

/** Checks whether `path` lies under `root` (case-insensitive, Windows-style). */
export function isUnder(path: string, root: string): boolean {
  const p = path.replace(/\//g, '\\').toLowerCase();
  const r = root.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase();
  return p === r || p.startsWith(r + '\\');
}

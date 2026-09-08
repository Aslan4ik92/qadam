// Builds nothing itself: expects `npm run build` to have produced dist/. Serves dist with vite preview,
// drives the browser-mock UI and saves PNGs for visual review.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const PORT = 4173;
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = process.env.SHOTS_DIR ?? '/tmp/claude-0/-home-user-qadam/ea13806c-211d-5e6f-a0d7-006a9638a8f8/scratchpad/shots';
mkdirSync(OUT, { recursive: true });

function findChromium() {
  if (process.env.CHROMIUM_PATH) return process.env.CHROMIUM_PATH;
  const base = process.env.PLAYWRIGHT_BROWSERS_PATH ?? '/opt/pw-browsers';
  try {
    for (const dir of readdirSync(base)) {
      if (dir.startsWith('chromium-')) {
        const p = path.join(base, dir, 'chrome-linux', 'chrome');
        if (existsSync(p)) return p;
      }
    }
    for (const dir of readdirSync(base)) {
      if (dir.startsWith('chromium_headless_shell-')) {
        const p = path.join(base, dir, 'chrome-linux', 'headless_shell');
        if (existsSync(p)) return p;
      }
    }
  } catch { /* fall through */ }
  return undefined;
}

async function waitForServer(url, ms = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try { const r = await fetch(url); if (r.ok) return; } catch { /* retry */ }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Server did not start at ${url}`);
}

const server = spawn(process.execPath, [path.join(root, 'node_modules', 'vite', 'bin', 'vite.js'), 'preview', '--port', String(PORT), '--strictPort', '--host', '127.0.0.1'], {
  cwd: root, stdio: ['ignore', 'pipe', 'pipe']
});
server.stderr.on('data', (d) => process.stderr.write(d));

try {
  await waitForServer(BASE);
  const browser = await chromium.launch({ executablePath: findChromium(), args: ['--disable-gpu'] });
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 }, deviceScaleFactor: 1, locale: 'ru-RU' });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => console.error('[pageerror]', e.message));
  page.on('console', (m) => { if (m.type() === 'error') console.error('[console]', m.text()); });

  async function openWithQuery(params, query) {
    await page.goto(`${BASE}/?${params}`);
    await page.waitForSelector('input[type="search"]');
    await page.fill('input[type="search"]', query);
    await page.keyboard.press('Enter');
    await page.waitForSelector('[role="option"]');
    // wait for preview text
    await page.waitForSelector('.preview .text mark', { timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(300);
  }

  await openWithQuery('theme=light&lang=ru', 'договор аренды');
  await page.screenshot({ path: path.join(OUT, 'main-light.png') });
  console.log('saved main-light.png');

  await openWithQuery('theme=dark&lang=kk', 'договор аренды');
  await page.screenshot({ path: path.join(OUT, 'main-dark.png') });
  console.log('saved main-dark.png');

  await openWithQuery('theme=light&lang=ru', 'отчёт');
  await page.keyboard.press('Control+Comma');
  await page.waitForSelector('[role="dialog"]');
  await page.click('[role="tab"]:nth-child(3)'); // Indexing tab (h2 is the first child)
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(OUT, 'settings.png') });
  console.log('saved settings.png');

  await page.goto(`${BASE}/?theme=light&lang=en&empty=1`);
  await page.waitForSelector('.onboarding');
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, 'onboarding.png') });
  console.log('saved onboarding.png');

  await page.goto(`${BASE}/?theme=light&lang=ru&indexing=1`);
  await page.waitForSelector('input[type="search"]');
  await page.fill('input[type="search"]', 'аренда');
  await page.keyboard.press('Enter');
  await page.waitForSelector('[role="progressbar"]');
  await page.waitForTimeout(2800); // into the "extracting" phase
  await page.screenshot({ path: path.join(OUT, 'indexing.png') });
  console.log('saved indexing.png');

  await browser.close();
} finally {
  server.kill('SIGTERM');
}

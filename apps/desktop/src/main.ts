import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';

// Apply a sensible theme before the first paint to avoid a flash; settings override it after load.
try {
  const params = new URLSearchParams(window.location.search);
  const forced = params.get('theme');
  const dark = forced === 'dark' || (forced !== 'light' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
} catch { /* ignore */ }

const target = document.getElementById('app');
if (!target) throw new Error('#app root element not found');

const app = mount(App, { target });

export default app;

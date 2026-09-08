<script lang="ts" module>
  /** Minimal inline icon set (24x24 grid, stroke-based). */
  export const ICONS: Record<string, string> = {
    search: 'M10.5 3a7.5 7.5 0 1 1 0 15 7.5 7.5 0 0 1 0-15Zm0 2a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11Zm7.2 11.3 3.6 3.6-1.4 1.4-3.6-3.6 1.4-1.4Z',
    close: 'M6.4 5 12 10.6 17.6 5 19 6.4 13.4 12 19 17.6 17.6 19 12 13.4 6.4 19 5 17.6 10.6 12 5 6.4 6.4 5Z',
    check: 'M9.5 16.2 5.3 12l-1.4 1.4 5.6 5.6 12-12-1.4-1.4-10.6 10.6Z',
    chevronDown: 'M12 15.4 5.3 8.7l1.4-1.4L12 12.6l5.3-5.3 1.4 1.4-6.7 6.7Z',
    chevronUp: 'M12 8.6l6.7 6.7-1.4 1.4L12 11.4l-5.3 5.3-1.4-1.4L12 8.6Z',
    chevronLeft: 'M15.4 12l-6.7 6.7-1.4-1.4 5.3-5.3-5.3-5.3 1.4-1.4 6.7 6.7Z',
    chevronRight: 'M8.6 12l6.7-6.7 1.4 1.4-5.3 5.3 5.3 5.3-1.4 1.4-6.7-6.7Z',
    settings: 'M12 8.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7Zm0 2a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3Zm-1.6-8h3.2l.5 2.6c.5.2 1 .5 1.4.8l2.5-.9 1.6 2.8-2 1.7c.1.5.1 1 0 1.5l2 1.7-1.6 2.8-2.5-.9c-.4.3-.9.6-1.4.8l-.5 2.6h-3.2l-.5-2.6c-.5-.2-1-.5-1.4-.8l-2.5.9-1.6-2.8 2-1.7a6 6 0 0 1 0-1.5l-2-1.7 1.6-2.8 2.5.9c.4-.3.9-.6 1.4-.8l.5-2.6Zm1.6 3.5c-.3 0-.5 0-.8.1l-.5.1-.4-2.2h-.6l-.4 2.2-.5.1c-.7.2-1.3.6-1.9 1l-.4.4-2-.8-.3.5 1.6 1.4-.1.6a4.2 4.2 0 0 0 0 1.2l.1.6-1.6 1.4.3.5 2-.8.4.4c.6.5 1.2.8 1.9 1l.5.1.4 2.2h.6l.4-2.2.5-.1c.7-.2 1.3-.6 1.9-1l.4-.4 2 .8.3-.5-1.6-1.4.1-.6a4.2 4.2 0 0 0 0-1.2l-.1-.6 1.6-1.4-.3-.5-2 .8-.4-.4c-.6-.5-1.2-.8-1.9-1l-.5-.1-.4-2.2h-.6L12 6Z',
    refresh: 'M12 4a8 8 0 0 1 7.4 5H17v2h6V5h-2v2.6A10 10 0 0 0 2 12h2a8 8 0 0 1 8-8Zm8 8h2a10 10 0 0 1-19 4.4V19H1v-6h6v2H4.6A8 8 0 0 0 20 12Z',
    stop: 'M6 6h12v12H6z',
    folder: 'M3 5h6.5l2 2H21v12H3V5Zm2 2v10h14V9h-9.3l-2-2H5Z',
    folderOpen: 'M3 5h6.5l2 2H20v3h2l-2.5 9H3V5Zm2 2v9.4L6.6 12H18v-3h-7.3l-2-2H5Z',
    folderAdd: 'M3 5h6.5l2 2H21v5h-2V9h-9.3l-2-2H5v10h7v2H3V5Zm15 9h2v3h3v2h-3v3h-2v-3h-3v-2h3v-3Z',
    plus: 'M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6V5Z',
    minus: 'M5 11h14v2H5z',
    trash: 'M9 3h6l1 2h4v2h-1v13a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7H4V5h4l1-2Zm-2 4v13h10V7H7Zm2 2h2v9H9V9Zm4 0h2v9h-2V9Z',
    open: 'M14 3h7v7h-2V6.4l-8.3 8.3-1.4-1.4L17.6 5H14V3ZM5 5h6v2H7v10h10v-4h2v6H5V5Z',
    explorer: 'M3 4h18v16H3V4Zm2 2v3h14V6H5Zm0 5v7h14v-7H5Zm2 1h6v2H7v-2Z',
    copy: 'M8 3h11v13h-2V5H8V3ZM5 7h11v14H5V7Zm2 2v10h7V9H7Z',
    apps: 'M4 4h7v7H4V4Zm2 2v3h3V6H6Zm7-2h7v7h-7V4Zm2 2v3h3V6h-3ZM4 13h7v7H4v-7Zm2 2v3h3v-3H6Zm7-2h7v7h-7v-7Zm2 2v3h3v-3h-3Z',
    help: 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm0 12a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4Zm0-9.5c2.1 0 3.5 1.3 3.5 3 0 1.2-.6 1.9-1.6 2.6-.9.6-1.1.9-1.1 1.6v.3h-1.7v-.4c0-1.4.6-2.1 1.6-2.8.8-.5 1-.8 1-1.3 0-.7-.7-1.2-1.7-1.2-.9 0-1.6.5-1.8 1.4L8.6 8.4c.3-1.6 1.7-2.9 3.4-2.9Z',
    info: 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-1 6h2v7h-2v-7Zm1-3.5a1.25 1.25 0 1 1 0 2.5 1.25 1.25 0 0 1 0-2.5Z',
    warning: 'M12 2 23 21H1L12 2Zm0 4-7.6 13h15.2L12 6Zm-1 4h2v5h-2v-5Zm1 6a1.2 1.2 0 1 1 0 2.4 1.2 1.2 0 0 1 0-2.4Z',
    error: 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-3.5 3.1L12 10.6l3.5-3.5 1.4 1.4L13.4 12l3.5 3.5-1.4 1.4-3.5-3.5-3.5 3.5-1.4-1.4L10.6 12 7.1 8.5l1.4-1.4Z',
    success: 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm4.3 4.6 1.4 1.4-6.7 6.7-3.7-3.7 1.4-1.4 2.3 2.3 5.3-5.3Z',
    sidebar: 'M3 4h18v16H3V4Zm2 2v12h4V6H5Zm6 0v12h8V6h-8Z',
    preview: 'M3 4h18v16H3V4Zm2 2v12h8V6H5Zm10 0v12h4V6h-4Z',
    sort: 'M3 6h12v2H3V6Zm0 5h8v2H3v-2Zm0 5h5v2H3v-2Zm14-9h2v9.2l2.3-2.3 1.4 1.4-4.7 4.7-4.7-4.7 1.4-1.4 2.3 2.3V7Z',
    filter: 'M3 4h18v2.4L14 13.6V20l-4 1.5v-8L3 6.4V4Zm2 2v.4l6 6.4v6.4l0-0V12.8l6-6.4V6H5Z',
    text: 'M4 4h16v2H4V4Zm0 5h16v2H4V9Zm0 5h10v2H4v-2Zm0 5h16v2H4v-2Z',
    mono: 'M8 4v2H6v12h2v2H4V4h4Zm8 0h4v16h-4v-2h2V6h-2V4Zm-5.3 4h2.6l2.7 8h-1.9l-.6-1.8h-3l-.6 1.8H8l2.7-8Zm.3 4.8h2l-1-3-1 3Z',
    calendar: 'M7 2h2v2h6V2h2v2h4v17H3V4h4V2Zm-2 4v3h14V6H5Zm0 5v8h14v-8H5Zm2 2h3v3H7v-3Z',
    size: 'M3 3h18v18H3V3Zm2 2v14h14V5H5Zm2 2h4v2H9v2H7V7Zm10 10h-4v-2h2v-2h2v4Z',
    clock: 'M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-1 3h2v5.2l3.5 2-1 1.7L11 13V7Z',
    dot: 'M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8Z',
    drive: 'M3 5h18v14H3V5Zm2 2v10h14V7H5Zm10 6.5a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Z',
    logs: 'M5 3h11l4 4v14H5V3Zm2 2v14h10V8h-3V5H7Zm2 6h6v1.6H9V11Zm0 3h6v1.6H9V14Z',
    more: 'M6 10.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Zm6 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Zm6 0a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3Z',
    eye: 'M12 5c5 0 9 3.5 10.5 7-1.5 3.5-5.5 7-10.5 7S3 15.5 1.5 12C3 8.5 7 5 12 5Zm0 2C8.3 7 5.2 9.4 3.7 12c1.5 2.6 4.6 5 8.3 5s6.8-2.4 8.3-5C18.8 9.4 15.7 7 12 7Zm0 2a3 3 0 1 1 0 6 3 3 0 0 1 0-6Z',
    // File categories
    fileDoc: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm2 8h6v1.6h-6V12Zm0 3h6v1.6h-6V15Zm0-6h3v1.6h-3V9Z',
    fileSheet: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm1 7h8v7H9v-7Zm1.5 1.5v1.2h2v-1.2h-2Zm3.5 0v1.2h2v-1.2h-2Zm-3.5 2.7v1.3h2v-1.3h-2Zm3.5 0v1.3h2v-1.3h-2Z',
    fileSlides: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm1.5 7h7v5h-7v-5Zm1.5 1.5v2h4v-2h-4Z',
    filePdf: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm1.6 8.5c.7 0 1.2.5 1.2 1.2 0 .8-.5 1.3-1.3 1.3H9v1.4H7.9v-3.9h1.7Zm-.6 1v.6h.5c.3 0 .4-.1.4-.3s-.1-.3-.4-.3H9Zm3.1-1c1.3 0 2 .7 2 1.9 0 1.3-.8 2-2.1 2h-1.4v-3.9h1.5Zm-.4 1v1.9h.4c.6 0 .9-.3.9-1s-.3-.9-.9-.9h-.4Zm3.2-1h2.5v1h-1.4v.6h1.2v.9h-1.2v1.4h-1.1v-3.9Z',
    fileText: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm2 6h6v1.6h-6V10Zm0 3h6v1.6h-6V13Zm0 3h4v1.6h-4V16Z',
    fileCode: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm2.2 7.4 1.1 1.1-1.5 1.5 1.5 1.5-1.1 1.1-2.6-2.6 2.6-2.6Zm3.6 0 2.6 2.6-2.6 2.6-1.1-1.1 1.5-1.5-1.5-1.5 1.1-1.1Z',
    fileBook: 'M5 3h12a2 2 0 0 1 2 2v16H7a2 2 0 0 1-2-2V3Zm2 2v11.3c.3-.2.7-.3 1-.3h9V5H7Zm1 13c-.6 0-1 .4-1 1s.4 1 1 1h9v-2H8Z',
    fileWeb: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm5 6.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7Zm0 1.4c-.4.6-.7 1.3-.8 2.1h1.6c-.1-.8-.4-1.5-.8-2.1Zm-1.9 2.1c.1-.7.3-1.3.6-1.9a2.1 2.1 0 0 0-1.1 1.9h.5Zm3.3 0h.5a2.1 2.1 0 0 0-1.1-1.9c.3.6.5 1.2.6 1.9ZM11.6 15c.1.7.3 1.3.6 1.9a2.1 2.1 0 0 1-1.1-1.9h.5Zm1.4 2.1c.4-.6.7-1.3.8-2.1h-1.6c.1.8.4 1.5.8 2.1Zm1.4-2.1c-.1.7-.3 1.3-.6 1.9a2.1 2.1 0 0 0 1.1-1.9h-.5Z',
    fileData: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm1.5 6h7v1.4h-7V10Zm0 2.6h7V14h-7v-1.4Zm0 2.6h7v1.4h-7v-1.4Zm2-5.2v7.8h-1V10h1Zm3 0v7.8h-1V10h1Z',
    fileMail: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm1.5 7h7v5h-7v-5Zm1.2 1 2.3 1.6 2.3-1.6h-4.6Z',
    fileOther: 'M6 2h9l5 5v15H6V2Zm2 2v16h10V8h-4V4H8Zm3 8h2v2h-2v-2Zm0 3h2v2h-2v-2Z',
    logo: ''
  };

  export const CATEGORY_ICON: Record<string, string> = {
    document: 'fileDoc', spreadsheet: 'fileSheet', presentation: 'fileSlides', pdf: 'filePdf', text: 'fileText',
    code: 'fileCode', ebook: 'fileBook', web: 'fileWeb', data: 'fileData', email: 'fileMail', other: 'fileOther'
  };
</script>

<script lang="ts">
  interface Props { name: string; size?: number; class?: string; title?: string; style?: string }
  let { name, size = 16, class: cls = '', title, style }: Props = $props();
  const path = $derived(ICONS[name] ?? ICONS.fileOther);
</script>

{#if name === 'logo'}
  <svg class={cls} width={size} height={size} viewBox="0 0 32 32" aria-hidden={title ? undefined : 'true'} role={title ? 'img' : undefined} {style}>
    {#if title}<title>{title}</title>{/if}
    <rect width="32" height="32" rx="7" fill="var(--accent)" />
    <path d="M9 7h9l5 5v13H9z" fill="none" stroke="var(--accent-fg)" stroke-width="2" stroke-linejoin="round" />
    <circle cx="19" cy="19" r="4.5" fill="var(--accent)" stroke="var(--accent-fg)" stroke-width="2" />
    <path d="M22.5 22.5l4 4" stroke="var(--accent-fg)" stroke-width="2.4" stroke-linecap="round" />
  </svg>
{:else}
  <svg class={cls} width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden={title ? undefined : 'true'} role={title ? 'img' : undefined} {style}>
    {#if title}<title>{title}</title>{/if}
    <path d={path} />
  </svg>
{/if}

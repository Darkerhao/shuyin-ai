<script setup lang="ts">
const paths = {
  book: 'M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253',
  shelf: 'M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10',
  grid: 'M3 3h7v7H3V3ZM14 3h7v7h-7V3ZM3 14h7v7H3v-7ZM14 14h7v7h-7v-7Z',
  list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
  more: 'M5 12h.01M12 12h.01M19 12h.01',
  settings: 'M4 7h16M4 17h16M8 4v6M16 14v6',
  plus: 'M12 5v14M5 12h14',
  search: 'M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  arrow: 'M5 12h14M13 6l6 6-6 6',
  back: 'M19 12H5M11 6l-6 6 6 6',
  chevron: 'm9 5 7 7-7 7',
  sparkle: 'm12 3 2.6 6.4L21 12l-6.4 2.6L12 21l-2.6-6.4L3 12l6.4-2.6L12 3ZM20 2v4M18 4h4',
  bolt: 'M13 10V3L4 14h7v7l9-11h-7Z',
  pen: 'M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586',
  microphone: 'M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 100-6 3 3 0 000 6',
  image: 'M4 3h16a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM3 16l6-6 5 5 3-3 4 4M16 7h.01',
  headphones: 'M4 14v-3a8 8 0 0 1 16 0v3M4 12H3v8h4v-8H4ZM20 12h1v8h-4v-8h3Z',
  play: 'M8 5l11 7-11 7V5Z',
  pause: 'M6 5h4v14H6V5ZM14 5h4v14h-4V5Z',
  volume: 'm11 5-5 4H3v6h3l5 4V5ZM15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14',
  volumeOff: 'm11 5-5 4H3v6h3l5 4V5ZM16 9l5 6M21 9l-5 6',
  rewind: 'M4 4v6h6M4.4 10a8 8 0 1 1 1 8',
  forward: 'M20 4v6h-6M19.6 10a8 8 0 1 0-1 8',
  palette: 'M12 3a9 9 0 1 0 0 18h1a2 2 0 0 0 1.5-3.3 1.5 1.5 0 0 1 1.2-2.5H18a3 3 0 0 0 3-3 9 9 0 0 0-9-9ZM7 10h.01M10 6.5h.01M15 6.5h.01M18 10h.01',
  video: 'M3 5h18v14H3V5ZM10 9l5 3-5 3V9Z',
  upload: 'M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12',
  copy: 'M9 9h12v12H9V9ZM5 15H3V3h12v2',
  check: 'm5 12 4 4L19 6',
  close: 'm6 6 12 12M18 6 6 18',
  trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7',
  refresh: 'M20 7v5h-5M4 17v-5h5M6 6a8 8 0 0 1 13 2M18 18a8 8 0 0 1-13-2',
  folder: 'M3 5h6l2 3h10v12H3V5Z',
  shield: 'M12 3 3 7v5c0 5 9 9 9 9s9-4 9-9V7l-9-4Zm-4 9 3 3 5-6',
  eye: 'M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12ZM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  eyeOff: 'm3 3 18 18M10.6 5.1 12 5c6.4 0 10 7 10 7a20 20 0 0 1-3.1 4M6.1 6.1A20 20 0 0 0 2 12s3.6 7 10 7a12 12 0 0 0 5.9-1.5M9.9 9.9a3 3 0 0 0 4.2 4.2',
  clock: 'M12 8v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
  save: 'M5 3h12l4 4v14H3V3h2Zm2 0v6h10V3M7 21v-8h10v8',
  download: 'M12 3v12M7 10l5 5 5-5M4 16v5h16v-5',
  info: 'M12 11v6M12 7h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
} as const
defineProps<{ name: keyof typeof paths }>()
</script>

<template>
  <svg
    class="app-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.65"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name]" />
  </svg>
</template>

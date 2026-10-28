<script setup lang="ts">
const paths = {
  book: 'M12 6.5C9 4.5 5.5 4 2 5v14c3.5-1 7-.5 10 1.5M12 6.5C15 4.5 18.5 4 22 5v14c-3.5-1-7-.5-10 1.5V6.5Z',
  shelf: 'M4 4v16M8 4v16M12 4v16M16 5l4 14M3 20h18',
  grid: 'M3 3h7v7H3V3ZM14 3h7v7h-7V3ZM3 14h7v7H3v-7ZM14 14h7v7h-7v-7Z',
  list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
  settings: 'M4 7h16M4 17h16M8 4v6M16 14v6',
  plus: 'M12 5v14M5 12h14',
  search: 'M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  arrow: 'M5 12h14M13 6l6 6-6 6',
  back: 'M19 12H5M11 6l-6 6 6 6',
  chevron: 'm9 5 7 7-7 7',
  sparkle: 'm12 3 2.6 6.4L21 12l-6.4 2.6L12 21l-2.6-6.4L3 12l6.4-2.6L12 3ZM20 2v4M18 4h4',
  pen: 'm15 5 4 4M4 20l4-1L20 7a2.8 2.8 0 0 0-4-4L4 15v5ZM13 20h7',
  image: 'M4 3h16a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM3 16l6-6 5 5 3-3 4 4M16 7h.01',
  headphones: 'M4 14v-3a8 8 0 0 1 16 0v3M4 12H3v8h4v-8H4ZM20 12h1v8h-4v-8h3Z',
  upload: 'M12 16V3M7 8l5-5 5 5M4 15v6h16v-6',
  copy: 'M9 9h12v12H9V9ZM5 15H3V3h12v2',
  check: 'm5 12 4 4L19 6',
  close: 'm6 6 12 12M18 6 6 18',
  refresh: 'M20 7v5h-5M4 17v-5h5M6 6a8 8 0 0 1 13 2M18 18a8 8 0 0 1-13-2',
  folder: 'M3 5h6l2 3h10v12H3V5Z',
  shield: 'M12 3 3 7v5c0 5 9 9 9 9s9-4 9-9V7l-9-4Zm-4 9 3 3 5-6',
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

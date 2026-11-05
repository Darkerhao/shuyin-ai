import { createApp } from 'vue'
import App from './App.vue'
import { router } from './app/router'
import { initializeTheme } from './composables/useTheme'
import './style.css'

initializeTheme()
createApp(App).use(router).mount('#app')

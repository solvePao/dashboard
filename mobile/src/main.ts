import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { useTheme } from './composables/useTheme'
import { logger } from './lib/logger'

// Apply persisted theme before first paint
useTheme()

logger.info('app', 'mobile frontend initialized')

createApp(App).mount('#app')

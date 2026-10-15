import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { useTheme } from './composables/useTheme'
import { logger } from './lib/logger'

// Apply persisted theme before first paint (index.html script already
// set the class; this keeps Vue state in sync and registers listeners).
useTheme()

logger.info('app', 'frontend initialized')

createApp(App).mount('#app')

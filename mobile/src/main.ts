import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { useDeviceAuth } from './composables/useDeviceAuth'
import { useTheme } from './composables/useTheme'
import { logger } from './lib/logger'

async function startApp(): Promise<void> {
  useTheme()
  await useDeviceAuth().init()
  createApp(App).mount('#app')
  logger.info('app', 'mobile frontend initialized')
}

void startApp()

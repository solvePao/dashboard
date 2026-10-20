<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Settings } from "@lucide/vue";
import HorizontalSplitLayout from "./components/HorizontalSplitLayout.vue";
import DeviceLeftPanel from "./components/DeviceLeftPanel.vue";
import DeviceAuthCard from "./components/DeviceAuthCard.vue";
import DeviceSettingsModal from "./components/DeviceSettingsModal.vue";
import { useDeviceAuth } from "./composables/useDeviceAuth";
import { useOrientationLock } from "./composables/useOrientationLock";
import type { SplitRatio } from "./types/layout";

const ratio: SplitRatio = "50-50";
const { init, isAuthenticated } = useDeviceAuth();
useOrientationLock();

const showSettingsModal = ref(false);

onMounted(() => {
  void init();
});
</script>

<template>
  <div class="mo-app-root orientation-locked-container bg-background text-foreground relative">
    <HorizontalSplitLayout :ratio="ratio">
      <!-- Left Box -->
      <template #left>
        <DeviceLeftPanel />
      </template>

      <!-- Right Box -->
      <template #right>
        <DeviceAuthCard />
      </template>
    </HorizontalSplitLayout>

    <!-- Floating Settings Button (Bottom Left) -->
    <button
      v-if="isAuthenticated"
      type="button"
      class="mo-floating-settings-btn fixed bottom-3.5 left-3.5 z-40 flex h-9 w-9 items-center justify-center rounded-full border border-border bg-card/90 text-muted-foreground shadow-lg backdrop-blur-xs transition hover:bg-accent hover:text-foreground active:scale-95"
      aria-label="Open settings and profile"
      title="Settings & Profile"
      @click="showSettingsModal = true"
    >
      <Settings class="h-4.5 w-4.5" />
    </button>

    <!-- Settings & Profile Modal -->
    <DeviceSettingsModal
      :open="showSettingsModal"
      @close="showSettingsModal = false"
    />
  </div>
</template>

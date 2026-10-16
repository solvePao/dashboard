<script setup lang="ts">
import { onMounted } from "vue";
import HorizontalSplitLayout from "./components/HorizontalSplitLayout.vue";
import DeviceInstructionsCard from "./components/DeviceInstructionsCard.vue";
import DeviceAuthCard from "./components/DeviceAuthCard.vue";
import { useDeviceAuth } from "./composables/useDeviceAuth";
import { useOrientationLock } from "./composables/useOrientationLock";
import type { SplitRatio } from "./types/layout";

const ratio: SplitRatio = "50-50";
const { init } = useDeviceAuth();
useOrientationLock();

onMounted(() => {
  void init();
});
</script>

<template>
  <div class="orientation-locked-container bg-background text-foreground">
    <HorizontalSplitLayout :ratio="ratio">
      <!-- Left Box -->
      <template #left>
        <DeviceInstructionsCard />
      </template>

      <!-- Right Box -->
      <template #right>
        <DeviceAuthCard />
      </template>
    </HorizontalSplitLayout>
  </div>
</template>

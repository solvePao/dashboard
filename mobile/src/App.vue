<script setup lang="ts">
import { onMounted, ref } from "vue";
import HorizontalSplitLayout from "./components/HorizontalSplitLayout.vue";
import DeviceInstructionsCard from "./components/DeviceInstructionsCard.vue";
import DeviceAuthCard from "./components/DeviceAuthCard.vue";
import { useDeviceAuth } from "./composables/useDeviceAuth";
import type { SplitRatio } from "./types/layout";

const ratio = ref<SplitRatio>("50-50");
const { init } = useDeviceAuth();

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

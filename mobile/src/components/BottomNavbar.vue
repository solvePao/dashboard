<script setup lang="ts">
import { useHealth } from "../composables/useHealth";

withDefaults(
  defineProps<{
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  }
);

const { status, version } = useHealth(2000);
</script>

<template>
  <div
    :class="[
      embedded
        ? 'flex items-center justify-between rounded-lg border border-border bg-muted/40 px-3 py-2 text-xs'
        : 'fixed bottom-2 left-1/2 z-40 -translate-x-1/2 rounded-full border border-border bg-card/90 px-4 py-1.5 shadow-md backdrop-blur-md',
    ]"
  >
    <div class="flex items-center gap-2.5">
      <!-- Status Dot -->
      <span class="relative flex h-2 w-2">
        <span
          v-if="status === 'online'"
          class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75"
        ></span>
        <span
          class="relative inline-flex h-2 w-2 rounded-full transition-colors duration-300"
          :class="status === 'online' ? 'bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]' : 'bg-rose-500 shadow-[0_0_6px_rgba(239,68,68,0.6)]'"
        ></span>
      </span>

      <span class="font-medium text-foreground">
        {{ status === "online" ? "Backend Connected" : "Backend Disconnected" }}
      </span>
    </div>

    <!-- Version -->
    <span class="font-mono text-[11px] text-muted-foreground">
      v({{ version }})
    </span>
  </div>
</template>

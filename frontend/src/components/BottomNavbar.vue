<script setup lang="ts">
import { useHealth } from "../composables/useHealth";

const { status, version } = useHealth();
</script>

<template>
  <footer
    class="fixed inset-x-0 bottom-0 z-40 flex h-[var(--statusbar-h)] items-center justify-center border-t border-border bg-background/80 pb-[env(safe-area-inset-bottom)] backdrop-blur-md"
  >
    <div class="flex items-center gap-2.5" role="status" :aria-label="`Backend ${status}`">
      <span class="font-mono text-[11px] font-medium leading-none text-muted-foreground">
        v({{ version }})
      </span>
      <span
        class="h-2 w-2 rounded-full"
        :class="
          status === 'online'
            ? 'bg-emerald-500 animate-pulse motion-reduce:animate-none'
            : 'bg-destructive'
        "
        aria-hidden="true"
      />
      <span class="sr-only">{{ status === "online" ? "Online" : "Offline" }}</span>
    </div>
  </footer>
</template>

<script setup lang="ts">
import type { GroupActivity } from "../../services/walkieTalkieActivity";

defineProps<{
  activity: GroupActivity[];
  switching: boolean;
}>();

defineEmits<{
  (e: "switch", groupId: string): void;
}>();
</script>

<template>
  <transition name="toast-slide">
    <div
      v-if="activity.length"
      class="w-full rounded-xl border border-primary/25 bg-card/95 p-2.5 shadow-lg backdrop-blur-md"
      role="status"
    >
      <div
        v-for="item in activity"
        :key="item.groupId"
        class="flex items-center justify-between gap-3 py-0.5 first:pt-0 last:pb-0"
      >
        <div class="flex items-center gap-2 min-w-0">
          <span class="relative flex h-2 w-2 shrink-0">
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
            <span class="relative inline-flex h-2 w-2 rounded-full bg-emerald-500" />
          </span>
          <p class="truncate text-xs text-foreground">
            Active in <span class="font-semibold">{{ item.name }}</span>
          </p>
        </div>
        <button
          type="button"
          :disabled="switching"
          class="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-sm transition hover:bg-primary/90 disabled:opacity-50 active:scale-95"
          @click="$emit('switch', item.groupId)"
        >
          Switch
        </button>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.toast-slide-enter-active,
.toast-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.toast-slide-enter-from,
.toast-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>


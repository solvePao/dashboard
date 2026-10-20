<script setup lang="ts">
import { Settings2, ShieldCheck } from "@lucide/vue";
import type { DeviceItem } from "../types/device";

defineProps<{
  device: DeviceItem;
}>();

const emit = defineEmits<{
  (e: "manage", device: DeviceItem): void;
}>();

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
</script>

<template>
  <div class="fe-devicerow-root flex flex-col items-start gap-3 py-3.5 sm:flex-row sm:items-center sm:justify-between">
    <div class="fe-devicerow-info flex min-w-0 flex-1 items-center gap-3">
      <span
        class="fe-devicerow-badge flex shrink-0 h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
      >
        <ShieldCheck class="fe-devicerow-badge-icon h-4 w-4" />
      </span>
      <div class="fe-devicerow-details min-w-0">
        <div class="fe-devicerow-name-row flex flex-wrap items-center gap-2">
          <span class="fe-devicerow-name break-words [overflow-wrap:anywhere] text-sm font-medium text-foreground">{{ device.device_name }}</span>
          <span
            class="fe-devicerow-status rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400 border border-emerald-500/20"
          >
            Authorized
          </span>
        </div>
        <p class="fe-devicerow-meta text-xs text-muted-foreground mt-0.5">
          Paired on {{ formatDate(device.created_at) }} · Valid until {{ formatDate(device.expires_at) }}
        </p>
      </div>
    </div>

    <button
      type="button"
      class="fe-devicerow-manage-btn inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-border px-2.5 py-1 text-xs font-medium text-foreground transition hover:bg-accent"
      @click="emit('manage', device)"
    >
      <Settings2 class="fe-devicerow-manage-icon h-3.5 w-3.5 text-primary" />
      <span class="fe-devicerow-manage-label">Manage</span>
    </button>
  </div>
</template>

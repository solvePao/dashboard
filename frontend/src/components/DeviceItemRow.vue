<script setup lang="ts">
import { RefreshCw, Settings2, ShieldCheck } from "@lucide/vue";
import type { DeviceItem } from "../types/device";

defineProps<{
  device: DeviceItem;
}>();

const emit = defineEmits<{
  (e: "manage", device: DeviceItem): void;
  (e: "repair", device: DeviceItem): void;
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
  <div class="fe-devicerow-root flex flex-col items-stretch gap-3 py-3.5 sm:flex-row sm:items-center sm:justify-between">
    <div class="fe-devicerow-info flex min-w-0 flex-1 items-start gap-3 sm:items-center">
      <span
        class="fe-devicerow-badge flex shrink-0 h-9 w-9 sm:h-8 sm:w-8 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
      >
        <ShieldCheck class="fe-devicerow-badge-icon h-4.5 w-4.5 sm:h-4 sm:w-4" />
      </span>
      <div class="fe-devicerow-details min-w-0 flex-1">
        <div class="fe-devicerow-name-row flex flex-wrap items-center gap-2">
          <span class="fe-devicerow-name break-words [overflow-wrap:anywhere] text-sm font-semibold sm:font-medium text-foreground">{{ device.device_name }}</span>
          <span
            class="fe-devicerow-status rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400 border border-emerald-500/20"
          >
            Authorized
          </span>
        </div>
        <p class="fe-devicerow-meta text-[11px] sm:text-xs text-muted-foreground mt-0.5">
          Paired {{ formatDate(device.created_at) }} · Valid until {{ formatDate(device.expires_at) }}
        </p>
      </div>
    </div>

    <div class="fe-devicerow-actions grid grid-cols-2 gap-2 w-full pt-1 sm:pt-0 sm:flex sm:w-auto sm:items-center sm:gap-2">
      <button
        type="button"
        class="fe-devicerow-repair-btn inline-flex min-h-[44px] sm:min-h-0 items-center justify-center gap-1.5 rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium text-foreground transition hover:bg-accent active:scale-[0.98] sm:px-2.5 sm:py-1"
        :title="`Re-pair ${device.device_name}`"
        @click="emit('repair', device)"
      >
        <RefreshCw class="fe-devicerow-repair-icon h-3.5 w-3.5 text-primary" />
        <span class="fe-devicerow-repair-label">Re-pair</span>
      </button>

      <button
        type="button"
        class="fe-devicerow-manage-btn inline-flex min-h-[44px] sm:min-h-0 items-center justify-center gap-1.5 rounded-lg border border-border bg-card px-3 py-2 text-xs font-semibold text-foreground transition hover:bg-accent active:scale-[0.98] sm:px-2.5 sm:py-1 sm:font-medium"
        @click="emit('manage', device)"
      >
        <Settings2 class="fe-devicerow-manage-icon h-3.5 w-3.5 text-primary" />
        <span class="fe-devicerow-manage-label">Manage</span>
      </button>
    </div>
  </div>
</template>

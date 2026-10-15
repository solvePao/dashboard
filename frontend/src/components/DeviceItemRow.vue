<script setup lang="ts">
import { Loader2, ShieldCheck, Trash2 } from "@lucide/vue";
import type { DeviceItem } from "../types/device";

defineProps<{
  device: DeviceItem;
  isRevoking: boolean;
}>();

const emit = defineEmits<{
  (e: "revoke", token: string): void;
}>();

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return iso;
  }
}
</script>

<template>
  <div class="flex items-center justify-between py-3.5">
    <div class="flex items-center gap-3">
      <span
        class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
      >
        <ShieldCheck class="h-4 w-4" />
      </span>
      <div>
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium text-foreground">{{ device.device_name }}</span>
          <span
            class="rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400 border border-emerald-500/20"
          >
            Authorized
          </span>
        </div>
        <p class="text-xs text-muted-foreground mt-0.5">
          Paired on {{ formatDate(device.created_at) }} · Valid until {{ formatDate(device.expires_at) }}
        </p>
      </div>
    </div>

    <button
      type="button"
      class="inline-flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1 text-xs text-destructive transition hover:bg-destructive/10"
      :disabled="isRevoking"
      @click="emit('revoke', device.token)"
    >
      <Loader2 v-if="isRevoking" class="h-3.5 w-3.5 animate-spin" />
      <Trash2 v-else class="h-3.5 w-3.5" />
      <span>Revoke</span>
    </button>
  </div>
</template>


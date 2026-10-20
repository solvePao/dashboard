<script setup lang="ts">
import { Loader2, Smartphone } from "@lucide/vue";
import { useDevices } from "../composables/useDevices";
import type { DeviceItem } from "../types/device";
import DeviceItemRow from "./DeviceItemRow.vue";

const emit = defineEmits<{
  (e: "add-device"): void;
  (e: "manage", device: DeviceItem): void;
}>();

const { devices, devicesLoading, devicesError } = useDevices();

function handleManage(device: DeviceItem): void {
  emit("manage", device);
}
</script>

<template>
  <div class="fe-devices-card rounded-xl border border-border bg-card p-5 text-card-foreground shadow-sm sm:p-6">
    <div class="fe-devices-header flex items-center justify-between border-b border-border/80 pb-4">
      <div class="fe-devices-header-left flex items-center gap-2.5">
        <span class="fe-devices-header-icon flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Smartphone class="fe-devices-header-glyph h-5 w-5" />
        </span>
        <div class="fe-devices-header-text">
          <h2 class="fe-devices-title text-base font-semibold leading-tight text-foreground">
            Connected Devices
          </h2>
          <p class="fe-devices-subtitle text-xs text-muted-foreground mt-0.5">
            Mobile devices authorized to access your account via secure tokens
          </p>
        </div>
      </div>

      <button
        type="button"
        class="fe-devices-add-btn inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-xs transition hover:bg-primary/90"
        @click="emit('add-device')"
      >
        <Smartphone class="fe-devices-add-btn-icon h-3.5 w-3.5" />
        <span class="fe-devices-add-btn-label">Add Device</span>
      </button>
    </div>

    <div
      v-if="devicesError"
      class="fe-devices-error mt-4 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive"
      role="alert"
    >
      {{ devicesError }}
    </div>

    <!-- Loading State -->
    <div v-if="devicesLoading" class="fe-devices-loading py-8 text-center text-xs text-muted-foreground">
      <Loader2 class="fe-devices-loading-icon mx-auto h-5 w-5 animate-spin text-primary mb-2" />
      Loading connected devices...
    </div>

    <!-- Empty State -->
    <div
      v-else-if="devices.length === 0"
      class="fe-devices-empty py-8 text-center text-xs text-muted-foreground"
    >
      <p class="fe-devices-empty-title font-medium text-foreground">No mobile devices connected</p>
      <p class="fe-devices-empty-hint mt-1 text-muted-foreground">
        Click "Add Device" to scan the QR code on your mobile frontend and pair your device.
      </p>
    </div>

    <!-- Device List -->
    <div v-else class="fe-devices-list divide-y divide-border/60 pt-2">
      <DeviceItemRow
        v-for="device in devices"
        :key="device.token"
        :device="device"
        @manage="handleManage"
      />
    </div>
  </div>
</template>

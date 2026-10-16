<script setup lang="ts">
import { ref } from "vue";
import { Loader2, Smartphone } from "@lucide/vue";
import { useDevices } from "../composables/useDevices";
import { useAuth } from "../composables/useAuth";
import DeviceItemRow from "./DeviceItemRow.vue";

const emit = defineEmits<{
  (e: "add-device"): void;
}>();

const { revokeDevice } = useAuth();
const { devices, devicesLoading, devicesError } = useDevices();
const revokingToken = ref<string | null>(null);
const revokeError = ref<string | null>(null);

async function handleRevoke(token: string): Promise<void> {
  revokeError.value = null;
  if (!window.confirm("Revoke this device? It will lose access immediately.")) return;
  revokingToken.value = token;
  try {
    const ok = await revokeDevice(token);
    if (!ok) {
      revokeError.value = devicesError.value || "Failed to revoke device. Please try again.";
    }
  } catch {
    revokeError.value = "Session expired. Please sign in again.";
  } finally {
    revokingToken.value = null;
  }
}
</script>

<template>
  <div class="rounded-xl border border-border bg-card p-5 text-card-foreground shadow-sm sm:p-6">
    <div class="flex items-center justify-between border-b border-border/80 pb-4">
      <div class="flex items-center gap-2.5">
        <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Smartphone class="h-5 w-5" />
        </span>
        <div>
          <h2 class="text-base font-semibold leading-tight text-foreground">
            Connected Devices
          </h2>
          <p class="text-xs text-muted-foreground mt-0.5">
            Mobile devices authorized to access your account via secure tokens
          </p>
        </div>
      </div>

      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground shadow-xs transition hover:bg-primary/90"
        @click="emit('add-device')"
      >
        <Smartphone class="h-3.5 w-3.5" />
        <span>Add Device</span>
      </button>
    </div>

    <div
      v-if="devicesError || revokeError"
      class="mt-4 rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive"
      role="alert"
    >
      {{ revokeError || devicesError }}
    </div>

    <!-- Loading State -->
    <div v-if="devicesLoading" class="py-8 text-center text-xs text-muted-foreground">
      <Loader2 class="mx-auto h-5 w-5 animate-spin text-primary mb-2" />
      Loading connected devices...
    </div>

    <!-- Empty State -->
    <div
      v-else-if="devices.length === 0"
      class="py-8 text-center text-xs text-muted-foreground"
    >
      <p class="font-medium text-foreground">No mobile devices connected</p>
      <p class="mt-1 text-muted-foreground">
        Click "Add Device" to scan the QR code on your mobile frontend and pair your device.
      </p>
    </div>

    <!-- Device List -->
    <div v-else class="divide-y divide-border/60 pt-2">
      <DeviceItemRow
        v-for="device in devices"
        :key="device.token"
        :device="device"
        :is-revoking="revokingToken === device.token"
        @revoke="handleRevoke"
      />
    </div>
  </div>
</template>

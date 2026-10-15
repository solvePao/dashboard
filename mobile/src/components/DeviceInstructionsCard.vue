<script setup lang="ts">
import { Loader2, RefreshCw, Smartphone } from "@lucide/vue";
import ThemeToggle from "./ThemeToggle.vue";
import DeviceLinkedSummary from "./DeviceLinkedSummary.vue";
import DevicePairingSteps from "./DevicePairingSteps.vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";

const {
  authState,
  statusMessage,
  isAuthenticated,
  checkAuth,
  disconnect,
} = useDeviceAuth();
</script>

<template>
  <div class="flex h-full w-full flex-col justify-between">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-border/60 pb-3">
      <div class="flex items-center gap-2">
        <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Smartphone class="h-4 w-4" />
        </span>
        <span class="text-sm font-semibold tracking-tight text-foreground">
          {{ isAuthenticated ? "Device Linked" : "Device Pairing" }}
        </span>
      </div>
      <ThemeToggle />
    </div>

    <!-- Body: Instructions or Device Info -->
    <div class="my-auto flex flex-col gap-3 py-3">
      <DeviceLinkedSummary v-if="isAuthenticated" />
      <DevicePairingSteps
        v-else
        :auth-state="authState"
        :status-message="statusMessage"
      />
    </div>

    <!-- Bottom: Action Button -->
    <div class="border-t border-border/60 pt-3">
      <div v-if="isAuthenticated">
        <button
          type="button"
          class="w-full inline-flex items-center justify-center gap-2 rounded-lg border border-border bg-card px-4 py-2.5 text-xs font-medium text-foreground shadow-sm transition hover:bg-accent"
          @click="disconnect"
        >
          Disconnect Device
        </button>
      </div>

      <div v-else>
        <button
          type="button"
          class="w-full inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground shadow-sm transition hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
          :disabled="authState === 'checking' || authState === 'loading'"
          @click="checkAuth"
        >
          <Loader2 v-if="authState === 'checking'" class="h-4 w-4 animate-spin" />
          <RefreshCw v-else class="h-4 w-4" />
          <span>{{ authState === "checking" ? "Checking Authorization..." : "Check for Auth" }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

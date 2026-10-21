<script setup lang="ts">
import { Loader2, RefreshCw, Smartphone } from "@lucide/vue";
import DeviceLinkedSummary from "./DeviceLinkedSummary.vue";
import DevicePairingSteps from "./DevicePairingSteps.vue";
import { useDeviceAuth } from "../composables/useDeviceAuth";

const {
  authState,
  statusMessage,
  isDisconnecting,
  isConfirmingDisconnect,
  disconnectError,
  isAuthenticated,
  checkAuth,
  createSession,
  requestDisconnect,
  cancelDisconnect,
  disconnect,
} = useDeviceAuth();
</script>

<template>
  <div class="mo-instr-root flex h-full w-full flex-col justify-between">
    <!-- Header -->
    <div class="mo-instr-header flex items-center justify-between border-b border-border/60 pb-3">
      <div class="mo-instr-header-left flex items-center gap-2">
        <span class="mo-instr-header-icon flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Smartphone class="mo-instr-header-glyph h-4 w-4" />
        </span>
        <span class="mo-instr-title text-sm font-semibold tracking-tight text-foreground">
          {{ isAuthenticated ? "Device Linked" : "Device Pairing" }}
        </span>
      </div>
    </div>

    <!-- Body: Instructions or Device Info -->
    <div class="mo-instr-body my-auto flex flex-col gap-3 py-3">
      <DeviceLinkedSummary v-if="isAuthenticated" />
      <DevicePairingSteps
        v-else
        :auth-state="authState"
        :status-message="statusMessage"
        @refresh="createSession"
        @check="checkAuth"
      />
    </div>

    <!-- Bottom: Action Button -->
    <div class="mo-instr-footer border-t border-border/60 pt-3">
      <div v-if="isAuthenticated" class="mo-instr-authed-actions">
        <button
          v-if="!isConfirmingDisconnect"
          type="button"
          class="mo-instr-disconnect-btn w-full inline-flex items-center justify-center gap-2 rounded-lg border border-border bg-card px-4 py-2.5 text-xs font-medium text-foreground shadow-sm transition hover:bg-accent"
          @click="requestDisconnect"
        >
          Disconnect Device
        </button>
        <div v-else class="mo-instr-confirm-wrap flex flex-col gap-2">
          <p v-if="disconnectError" class="mo-instr-disconnect-error text-xs font-medium text-destructive">
            {{ disconnectError }}
          </p>
          <div class="mo-instr-confirm-row flex items-center gap-2">
            <button
              type="button"
              :disabled="isDisconnecting"
              class="mo-instr-confirm-btn flex-1 inline-flex items-center justify-center gap-2 rounded-lg bg-destructive px-4 py-2.5 text-xs font-medium text-white shadow-sm transition hover:bg-destructive/90 disabled:opacity-50"
              @click="disconnect"
            >
              {{ isDisconnecting ? "Disconnecting..." : "Confirm Disconnect" }}
            </button>
            <button
              type="button"
              :disabled="isDisconnecting"
              class="mo-instr-cancel-btn inline-flex items-center justify-center gap-2 rounded-lg border border-border bg-card px-4 py-2.5 text-xs font-medium text-foreground shadow-sm transition hover:bg-accent disabled:opacity-50"
              @click="cancelDisconnect"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>

      <div v-else class="mo-instr-check-row flex items-center justify-between">
        <span class="mo-instr-check-status text-[11px] text-muted-foreground">
          {{ authState === "checking" ? "Checking…" : "Waiting for authorization…" }}
        </span>
        <button
          type="button"
          class="mo-instr-check-btn inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-border bg-card text-foreground shadow-sm transition hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
          :disabled="authState === 'checking' || authState === 'loading'"
          aria-label="Check for Auth"
          title="Check for Auth"
          @click="checkAuth"
        >
          <Loader2 v-if="authState === 'checking' || authState === 'loading'" class="mo-instr-check-loading-icon h-4 w-4 animate-spin" />
          <RefreshCw v-else class="mo-instr-check-icon h-4 w-4" />
        </button>
      </div>
    </div>
  </div>
</template>

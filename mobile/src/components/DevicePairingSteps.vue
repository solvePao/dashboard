<script setup lang="ts">
import { computed } from "vue";
import { AlertCircle, Laptop, Plus, Radio, RefreshCw, ScanLine } from "@lucide/vue";
import type { AuthState } from "../types/auth";

const props = defineProps<{
  authState: AuthState;
  statusMessage: string;
}>();

const emit = defineEmits<{
  (e: "refresh"): void;
  (e: "check"): void;
}>();

const isNotableStatusMessage = computed(() => {
  if (!props.statusMessage) return false;
  if (props.authState === "expired" || props.authState === "error") return true;
  const lower = props.statusMessage.toLowerCase();
  return (
    lower.includes("revoked") ||
    lower.includes("error") ||
    lower.includes("failed") ||
    lower.includes("cannot reach")
  );
});
</script>

<template>
  <div class="mo-steps-root flex flex-col gap-2 sm:gap-2.5 w-full">
    <!-- Intro / Cue -->
    <div class="flex items-center justify-between">
      <p class="mo-steps-intro text-[11px] sm:text-xs font-medium text-muted-foreground">
        Connect this companion screen in 3 quick steps:
      </p>
    </div>

    <!-- 3 Actionable Step Cards -->
    <div class="mo-steps-list space-y-1.5 sm:space-y-2">
      <!-- Step 1 -->
      <div
        class="mo-steps-item-1 mo-step-card flex items-center gap-2.5 rounded-xl border border-border/70 bg-card/60 p-2 sm:p-2.5 transition hover:bg-accent/40 shadow-2xs"
      >
        <span
          class="mo-steps-num-1 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
        >
          <Laptop class="h-3.5 w-3.5" />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-1.5">
            <span class="font-mono text-[10px] font-semibold text-primary">01</span>
            <span class="mo-steps-text-1 text-xs font-semibold text-foreground tracking-tight">Open Web Dashboard</span>
          </div>
          <p class="text-[11px] text-muted-foreground truncate">
            Sign in with Google on your computer or tablet
          </p>
        </div>
      </div>

      <!-- Step 2 -->
      <div
        class="mo-steps-item-2 mo-step-card flex items-center gap-2.5 rounded-xl border border-border/70 bg-card/60 p-2 sm:p-2.5 transition hover:bg-accent/40 shadow-2xs"
      >
        <span
          class="mo-steps-num-2 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
        >
          <Plus class="h-3.5 w-3.5" />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-1.5">
            <span class="font-mono text-[10px] font-semibold text-primary">02</span>
            <span class="mo-steps-text-2 text-xs font-semibold text-foreground tracking-tight">Click "Add Device"</span>
          </div>
          <p class="text-[11px] text-muted-foreground truncate">
            Found in the top navigation bar of the dashboard
          </p>
        </div>
      </div>

      <!-- Step 3 -->
      <div
        class="mo-steps-item-3 mo-step-card flex items-center gap-2.5 rounded-xl border border-border/70 bg-card/60 p-2 sm:p-2.5 transition hover:bg-accent/40 shadow-2xs"
      >
        <span
          class="mo-steps-num-3 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
        >
          <ScanLine class="h-3.5 w-3.5" />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-1.5">
            <span class="font-mono text-[10px] font-semibold text-primary">03</span>
            <span class="mo-steps-text-3 text-xs font-semibold text-foreground tracking-tight">Scan QR or Enter Code</span>
          </div>
          <p class="text-[11px] text-muted-foreground truncate">
            Point camera at the QR or type the 6-character code
          </p>
        </div>
      </div>
    </div>

    <!-- Real-time Auto-Pairing Notice -->
    <div
      v-if="authState !== 'expired' && authState !== 'error'"
      class="mo-steps-live-cue flex items-center gap-2 rounded-lg border border-primary/20 bg-primary/5 px-2.5 py-1.5 text-[11px] text-primary"
    >
      <Radio class="h-3.5 w-3.5 shrink-0 animate-pulse text-primary" />
      <span class="truncate">Auto-pairs in real time — keep this screen open while approving</span>
    </div>

    <!-- Expired Warning Banner with Action -->
    <div
      v-else-if="authState === 'expired'"
      class="mo-steps-feedback mo-steps-expired-cue flex items-center justify-between gap-2 rounded-xl border border-destructive/20 bg-destructive/10 p-2 sm:p-2.5 text-xs text-destructive"
    >
      <div class="flex items-center gap-2 min-w-0">
        <AlertCircle class="h-4 w-4 shrink-0 text-destructive" />
        <span class="text-[11px] truncate">Pairing code expired. Refresh to generate a new code.</span>
      </div>
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded bg-destructive px-2 py-1 text-[11px] font-medium text-white shadow-2xs transition hover:bg-destructive/90 shrink-0 active:scale-95"
        @click="emit('refresh')"
      >
        <RefreshCw class="h-3 w-3" />
        <span>Regenerate</span>
      </button>
    </div>

    <!-- Error or Special Status Banner -->
    <div
      v-else-if="isNotableStatusMessage"
      class="mo-steps-feedback mo-steps-status-cue flex items-center justify-between gap-2 rounded-xl border border-destructive/20 bg-destructive/10 p-2 sm:p-2.5 text-xs text-destructive"
    >
      <div class="flex items-center gap-2 min-w-0">
        <AlertCircle class="h-4 w-4 shrink-0 text-destructive" />
        <span class="text-[11px] truncate">{{ statusMessage }}</span>
      </div>
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded bg-destructive px-2 py-1 text-[11px] font-medium text-white shadow-2xs transition hover:bg-destructive/90 shrink-0 active:scale-95"
        @click="emit('refresh')"
      >
        <RefreshCw class="h-3 w-3" />
        <span>Retry</span>
      </button>
    </div>
  </div>
</template>

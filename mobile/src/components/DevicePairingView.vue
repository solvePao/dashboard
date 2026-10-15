<script setup lang="ts">
import { ref } from "vue";
import { Check, Copy, QrCode, RefreshCw } from "@lucide/vue";
import type { AuthState } from "../types/auth";

const props = defineProps<{
  authState: AuthState;
  qrSvg: string;
  shortCode: string;
}>();

const emit = defineEmits<{
  (e: "refresh"): void;
}>();

const copied = ref(false);

function copyCode(): void {
  if (!props.shortCode) return;
  navigator.clipboard.writeText(props.shortCode).catch(() => {});
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}
</script>

<template>
  <div class="flex flex-1 flex-col items-center justify-center p-2 text-center">
    <div class="mb-2 flex items-center gap-1.5 text-xs font-medium uppercase tracking-wider text-muted-foreground">
      <QrCode class="h-4 w-4" />
      <span>Scan or Enter Code</span>
    </div>

    <!-- QR Code Display -->
    <div
      class="relative flex items-center justify-center rounded-2xl border border-border/80 bg-white p-3 shadow-sm transition hover:shadow-md dark:bg-white"
      style="width: 210px; height: 210px"
    >
      <!-- Loading overlay -->
      <div
        v-if="authState === 'loading'"
        class="flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <RefreshCw class="h-6 w-6 animate-spin text-primary" />
        <span>Generating QR...</span>
      </div>

      <!-- Render Backend SVG directly -->
      <div
        v-else-if="qrSvg"
        class="h-full w-full flex items-center justify-center overflow-hidden [&>svg]:h-full [&>svg]:w-full"
        v-html="qrSvg"
      ></div>

      <!-- Expired Overlay -->
      <div
        v-else-if="authState === 'expired'"
        class="flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <p class="font-semibold text-destructive">QR Expired</p>
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded bg-primary px-2.5 py-1 text-xs text-primary-foreground"
          @click="emit('refresh')"
        >
          <RefreshCw class="h-3 w-3" />
          Regenerate
        </button>
      </div>
    </div>

    <!-- 6-Character Short Pairing Code -->
    <div class="mt-4 flex flex-col items-center gap-1.5">
      <span class="text-xs text-muted-foreground">Pairing Code (6 Characters)</span>
      <div class="flex items-center gap-2">
        <div
          class="flex items-center justify-center rounded-lg border border-primary/30 bg-primary/5 px-4 py-1.5 font-mono text-xl font-bold tracking-widest text-primary shadow-sm"
        >
          {{ shortCode || "------" }}
        </div>
        <button
          type="button"
          class="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground transition hover:bg-accent hover:text-foreground"
          :title="copied ? 'Copied' : 'Copy code'"
          @click="copyCode"
        >
          <Check v-if="copied" class="h-4 w-4 text-emerald-500" />
          <Copy v-else class="h-4 w-4" />
        </button>
      </div>
    </div>

    <!-- Regenerate Button -->
    <div class="mt-3 flex items-center gap-2">
      <button
        type="button"
        class="inline-flex items-center gap-1 text-[11px] text-muted-foreground transition hover:text-foreground"
        :disabled="authState === 'loading'"
        @click="emit('refresh')"
      >
        <RefreshCw class="h-3 w-3" :class="authState === 'loading' && 'animate-spin'" />
        Refresh Code
      </button>
    </div>

    <p class="mt-2 text-[11px] text-muted-foreground/70">
      Code expires in 15 minutes. Derived from 128-hex session ID.
    </p>
  </div>
</template>


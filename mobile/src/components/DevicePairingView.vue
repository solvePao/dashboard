<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Check, Copy, QrCode, RefreshCw } from "@lucide/vue";
import type { AuthState } from "../types/auth";
import { formatExpiryCountdown } from "../lib/format";

const props = defineProps<{
  authState: AuthState;
  qrSvg: string;
  shortCode: string;
  expiresAt: string;
}>();

const emit = defineEmits<{
  (e: "refresh"): void;
}>();

const copied = ref(false);

// Render backend SVG through <img> so embedded scripts can never execute.
const qrImgSrc = computed(() =>
  props.qrSvg ? `data:image/svg+xml;utf8,${encodeURIComponent(props.qrSvg)}` : "",
);

const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | undefined;

onMounted(() => {
  timer = setInterval(() => {
    now.value = Date.now();
  }, 1000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

const expiryMs = computed(() => {
  const parsed = Date.parse(props.expiresAt);
  return Number.isNaN(parsed) ? null : parsed;
});

const countdownText = computed(() =>
  expiryMs.value === null ? "" : formatExpiryCountdown(expiryMs.value - now.value),
);

async function copyCode(): Promise<void> {
  if (!props.shortCode) return;
  try {
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(props.shortCode);
    } else {
      throw new Error("Clipboard API unavailable");
    }
  } catch {
    // Fallback for non-secure contexts where navigator.clipboard is missing.
    try {
      const area = document.createElement("textarea");
      area.value = props.shortCode;
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    } catch {
      // Clipboard unavailable; the code stays visible for manual entry.
    }
  }
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}
</script>

<template>
  <div class="mo-pairing-root flex flex-1 flex-col items-center justify-center p-4 sm:p-5 text-center">
    <div class="mo-pairing-header mb-2 flex items-center gap-1.5 text-xs font-medium uppercase tracking-wider text-muted-foreground">
      <QrCode class="mo-pairing-header-icon h-4 w-4" />
      <span class="mo-pairing-header-label">Scan or Enter Code</span>
    </div>

    <!-- QR Code Display -->
    <div
      class="mo-pairing-qr-box relative flex items-center justify-center rounded-2xl border border-border/80 bg-white p-3 shadow-sm transition hover:shadow-md dark:bg-white"
      style="width: 210px; height: 210px"
    >
      <!-- Expired Overlay -->
      <div
        v-if="authState === 'expired'"
        class="mo-pairing-expired flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <p class="mo-pairing-expired-title font-semibold text-destructive">QR Expired</p>
        <button
          type="button"
          class="mo-pairing-regen-btn inline-flex items-center gap-1 rounded bg-primary px-2.5 py-1 text-xs text-primary-foreground"
          @click="emit('refresh')"
        >
          <RefreshCw class="mo-pairing-regen-icon h-3 w-3" />
          Regenerate
        </button>
      </div>

      <!-- Loading overlay -->
      <div
        v-else-if="authState === 'loading'"
        class="mo-pairing-loading flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <RefreshCw class="mo-pairing-loading-icon h-6 w-6 animate-spin text-primary" />
        <span class="mo-pairing-loading-text">Generating QR...</span>
      </div>

      <!-- Backend SVG via <img>: scripts inside cannot execute -->
      <img
        v-else-if="qrImgSrc"
        :src="qrImgSrc"
        alt="Device pairing QR code"
        class="mo-pairing-qr-img h-full w-full"
      />

      <!-- Error overlay -->
      <div
        v-else-if="authState === 'error'"
        class="mo-pairing-error flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <p class="mo-pairing-error-title font-semibold text-destructive">Could not load QR</p>
        <button
          type="button"
          class="mo-pairing-retry-btn inline-flex items-center gap-1 rounded bg-primary px-2.5 py-1 text-xs text-primary-foreground"
          @click="emit('refresh')"
        >
          <RefreshCw class="mo-pairing-retry-icon h-3 w-3" />
          Try Again
        </button>
      </div>

      <!-- Idle placeholder -->
      <div
        v-else
        class="mo-pairing-idle flex flex-col items-center justify-center gap-2 text-xs text-neutral-600"
      >
        <span class="mo-pairing-idle-text">Preparing session...</span>
      </div>
    </div>

    <!-- 6-Character Short Pairing Code -->
    <div class="mo-pairing-code-block mt-4 flex flex-col items-center gap-1.5">
      <span class="mo-pairing-code-label text-xs text-muted-foreground">Pairing Code (6 Characters)</span>
      <div class="mo-pairing-code-row flex items-center gap-2">
        <div
          class="mo-pairing-code-value flex items-center justify-center rounded-lg border border-primary/30 bg-primary/5 px-4 py-1.5 font-mono text-xl font-bold tracking-widest text-primary shadow-sm"
        >
          {{ shortCode || "------" }}
        </div>
        <button
          type="button"
          class="mo-pairing-copy-btn inline-flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground transition hover:bg-accent hover:text-foreground"
          :title="copied ? 'Copied' : 'Copy code'"
          @click="copyCode"
        >
          <Check v-if="copied" class="mo-pairing-copied-icon h-4 w-4 text-emerald-500" />
          <Copy v-else class="mo-pairing-copy-icon h-4 w-4" />
        </button>
      </div>
    </div>

    <!-- Regenerate Button -->
    <div class="mo-pairing-refresh-row mt-3 flex items-center gap-2">
      <button
        type="button"
        class="mo-pairing-refresh-btn inline-flex items-center gap-1 text-[11px] text-muted-foreground transition hover:text-foreground"
        :disabled="authState === 'loading'"
        @click="emit('refresh')"
      >
        <RefreshCw class="mo-pairing-refresh-icon h-3 w-3" :class="authState === 'loading' && 'animate-spin'" />
        Refresh Code
      </button>
    </div>

    <p v-if="countdownText" class="mo-pairing-countdown mt-2 text-[11px] text-muted-foreground/70">
      Code expires in {{ countdownText }}.
    </p>
    <p v-else class="mo-pairing-countdown-fallback mt-2 text-[11px] text-muted-foreground/70">
      Code expires in 15 minutes.
    </p>
  </div>
</template>

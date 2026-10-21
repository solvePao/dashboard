<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Check, Clock, Copy, QrCode, RefreshCw } from "@lucide/vue";
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
  <div class="mo-pairing-root flex h-full w-full flex-col justify-between p-4 sm:p-5 overflow-hidden">
    <!-- Header: Aligns with DeviceLeftPanel header -->
    <div class="mo-pairing-header flex items-center justify-between border-b border-border/60 pb-3">
      <div class="flex items-center gap-2">
        <span class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <QrCode class="h-4 w-4" />
        </span>
        <span class="text-sm font-semibold tracking-tight text-foreground">
          Pairing Code &amp; QR
        </span>
      </div>
    </div>

    <!-- Body: QR Code & 6-character code pill -->
    <div class="my-auto flex flex-1 flex-col items-center justify-center py-1 sm:py-2 gap-2 sm:gap-2.5 min-h-0 overflow-hidden">
      <!-- QR Code Display with responsive max bounds -->
      <div
        class="mo-pairing-qr-box relative flex items-center justify-center rounded-xl sm:rounded-2xl border border-border/80 bg-white p-2 shadow-xs transition hover:shadow-sm dark:bg-white shrink-0 aspect-square w-28 h-28 sm:w-32 sm:h-32 md:w-36 md:h-36 max-h-[35vh]"
      >
        <!-- Expired Overlay -->
        <div
          v-if="authState === 'expired'"
          class="mo-pairing-expired flex flex-col items-center justify-center gap-1.5 text-xs text-neutral-600"
        >
          <p class="mo-pairing-expired-title font-semibold text-destructive">QR Expired</p>
          <button
            type="button"
            class="mo-pairing-regen-btn inline-flex items-center gap-1 rounded bg-primary px-2 py-0.5 text-xs text-primary-foreground shadow-xs"
            @click="emit('refresh')"
          >
            <RefreshCw class="mo-pairing-regen-icon h-3 w-3" />
            Regenerate
          </button>
        </div>

        <!-- Loading overlay -->
        <div
          v-else-if="authState === 'loading'"
          class="mo-pairing-loading flex flex-col items-center justify-center gap-1.5 text-xs text-neutral-600"
        >
          <RefreshCw class="mo-pairing-loading-icon h-5 w-5 animate-spin text-primary" />
          <span class="mo-pairing-loading-text text-[11px]">Generating…</span>
        </div>

        <!-- Backend SVG via <img>: scripts inside cannot execute -->
        <img
          v-else-if="qrImgSrc"
          :src="qrImgSrc"
          alt="Device pairing QR code"
          class="mo-pairing-qr-img h-full w-full object-contain"
        />

        <!-- Error overlay -->
        <div
          v-else-if="authState === 'error'"
          class="mo-pairing-error flex flex-col items-center justify-center gap-1.5 text-xs text-neutral-600"
        >
          <p class="mo-pairing-error-title font-semibold text-destructive">Could not load QR</p>
          <button
            type="button"
            class="mo-pairing-retry-btn inline-flex items-center gap-1 rounded bg-primary px-2 py-0.5 text-xs text-primary-foreground shadow-xs"
            @click="emit('refresh')"
          >
            <RefreshCw class="mo-pairing-retry-icon h-3 w-3" />
            Try Again
          </button>
        </div>

        <!-- Idle placeholder -->
        <div
          v-else
          class="mo-pairing-idle flex flex-col items-center justify-center gap-1.5 text-xs text-neutral-600"
        >
          <span class="mo-pairing-idle-text text-[11px]">Preparing…</span>
        </div>
      </div>

      <!-- 6-Character Short Pairing Code -->
      <div class="mo-pairing-code-block flex flex-col items-center gap-1 shrink-0">
        <span class="mo-pairing-code-label text-[11px] text-muted-foreground">Pairing Code</span>
        <div class="mo-pairing-code-row flex items-center gap-2">
          <div
            class="mo-pairing-code-value flex items-center justify-center rounded-lg border border-primary/30 bg-primary/5 px-3.5 py-1 font-mono text-lg sm:text-xl font-bold tracking-widest text-primary shadow-xs"
          >
            {{ shortCode || "------" }}
          </div>
          <button
            type="button"
            class="mo-pairing-copy-btn inline-flex h-8 w-8 items-center justify-center rounded-lg border border-border bg-card text-muted-foreground transition hover:bg-accent hover:text-foreground active:scale-95 shadow-xs"
            :title="copied ? 'Copied' : 'Copy code'"
            @click="copyCode"
          >
            <Check v-if="copied" class="mo-pairing-copied-icon h-3.5 w-3.5 text-emerald-500" />
            <Copy v-else class="mo-pairing-copy-icon h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>

    <!-- Footer: Expiry countdown on left, Refresh on right -->
    <div class="border-t border-border/60 pt-3 flex items-center justify-between text-[11px] text-muted-foreground">
      <span class="mo-pairing-countdown flex items-center gap-1.5">
        <Clock class="h-3 w-3 text-muted-foreground/80" />
        <span v-if="countdownText">Code expires in {{ countdownText }}</span>
        <span v-else>Code expires in 15m</span>
      </span>

      <button
        type="button"
        class="mo-pairing-refresh-btn inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium text-muted-foreground transition hover:bg-accent hover:text-foreground disabled:opacity-50"
        :disabled="authState === 'loading'"
        @click="emit('refresh')"
      >
        <RefreshCw class="mo-pairing-refresh-icon h-3 w-3" :class="authState === 'loading' && 'animate-spin'" />
        <span>Refresh Code</span>
      </button>
    </div>
  </div>
</template>

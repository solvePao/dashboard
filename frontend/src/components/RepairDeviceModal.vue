<script setup lang="ts">
import AppDialog from "./AppDialog.vue";
import { onBeforeUnmount, ref, watch, useTemplateRef } from "vue";
import { Camera, CheckCircle2, KeyRound, RefreshCw, X, Loader2 } from "@lucide/vue";
import { useAuth } from "../composables/useAuth";
import type { DeviceItem } from "../types/device";
import DeviceCameraScanner from "./DeviceCameraScanner.vue";

const props = defineProps<{
  open: boolean;
  device: DeviceItem | null;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const { reauthorizeDevice } = useAuth();

const activeTab = ref<"code" | "camera">("code");
const codeInput = ref("");
const loading = ref(false);
const resultMessage = ref<{ type: "success" | "error"; text: string } | null>(null);

const scannerRef = useTemplateRef<InstanceType<typeof DeviceCameraScanner>>("scannerRef");
let closeTimer: ReturnType<typeof setTimeout> | undefined;
let lastSubmitAt = 0;
let autoSubmittedCode = "";

function clearCloseTimer(): void {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = undefined;
  }
}

function normalizeCode(raw: string): string {
  const trimmed = raw.trim().replace(/\s+/g, "");
  return trimmed.length <= 12 ? trimmed.toUpperCase() : trimmed;
}

async function submitRepair(code: string): Promise<void> {
  if (!props.device) return;
  const trimmed = normalizeCode(code);
  if (!trimmed) {
    resultMessage.value = {
      type: "error",
      text: "Please enter the pairing code from your mobile device.",
    };
    return;
  }
  if (!/^[A-Za-z0-9]{6,32}$/.test(trimmed)) {
    resultMessage.value = {
      type: "error",
      text: "That code is invalid. Please enter or scan the code shown on your mobile device.",
    };
    return;
  }

  const now = Date.now();
  if (loading.value || now - lastSubmitAt < 1000) return;
  lastSubmitAt = now;

  loading.value = true;
  resultMessage.value = null;

  const res = await reauthorizeDevice(props.device.token, trimmed);
  loading.value = false;

  if (res.success) {
    resultMessage.value = { type: "success", text: res.message };
    codeInput.value = "";
    clearCloseTimer();
    closeTimer = setTimeout(() => {
      handleClose();
    }, 1500);
  } else {
    resultMessage.value = { type: "error", text: res.message };
  }
}

function handleClose(): void {
  clearCloseTimer();
  void scannerRef.value?.stopCamera();
  resultMessage.value = null;
  codeInput.value = "";
  emit("close");
}

watch(codeInput, (val) => {
  if (!props.open || activeTab.value !== "code" || loading.value) return;
  const normalized = normalizeCode(val);
  if ((normalized.length === 6 || normalized.length === 32) && /^[A-Za-z0-9]+$/.test(normalized)) {
    if (normalized !== autoSubmittedCode) {
      autoSubmittedCode = normalized;
      void submitRepair(normalized);
    }
  }
});

watch(
  () => props.open,
  (isOpen) => {
    autoSubmittedCode = "";
    if (isOpen) {
      resultMessage.value = null;
      codeInput.value = "";
      activeTab.value = "code";
    } else {
      clearCloseTimer();
      void scannerRef.value?.stopCamera();
    }
  }
);

onBeforeUnmount(() => {
  clearCloseTimer();
});
</script>

<template>
  <AppDialog :open="open && !!device" :title="`Re-pair ${device?.device_name || 'Device'}`" @close="handleClose">
    <!-- Header -->
    <div class="fe-repairmodal-header flex items-center justify-between border-b border-border/80 pb-3 sm:pb-4">
      <div class="fe-repairmodal-header-left flex min-w-0 items-center gap-2.5 pr-2">
        <span class="fe-repairmodal-header-icon flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <RefreshCw class="fe-repairmodal-header-glyph h-5 w-5" />
        </span>
        <div class="fe-repairmodal-header-text min-w-0">
          <h2 id="repair-device-title" class="fe-repairmodal-title text-base font-semibold leading-tight text-foreground truncate">
            Re-pair {{ device?.device_name }}
          </h2>
          <p class="fe-repairmodal-subtitle text-xs text-muted-foreground mt-0.5 truncate">
            Reconnect this device and restore its configured dashboard &amp; widgets
          </p>
        </div>
      </div>
      <button
        type="button"
        class="fe-repairmodal-close-btn flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition hover:bg-accent hover:text-foreground"
        aria-label="Close"
        @click="handleClose"
      >
        <X class="fe-repairmodal-close-icon h-4 w-4" />
      </button>
    </div>

    <!-- Tab Switcher -->
    <div class="fe-repairmodal-tabs mt-3 sm:mt-4 flex rounded-lg bg-muted p-1 text-xs">
      <button
        type="button"
        class="fe-repairmodal-tab-code flex flex-1 min-h-[40px] items-center justify-center gap-1.5 rounded-md py-2 font-medium transition"
        :class="activeTab === 'code' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
        @click="activeTab = 'code'"
      >
        <KeyRound class="fe-repairmodal-tab-code-icon h-3.5 w-3.5" />
        6-Character Code
      </button>
      <button
        type="button"
        class="fe-repairmodal-tab-camera flex flex-1 min-h-[40px] items-center justify-center gap-1.5 rounded-md py-2 font-medium transition"
        :class="activeTab === 'camera' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
        @click="activeTab = 'camera'"
      >
        <Camera class="fe-repairmodal-tab-camera-icon h-3.5 w-3.5" />
        Camera Scanner
      </button>
    </div>

    <!-- Result Banner -->
    <div
      v-if="resultMessage"
      class="fe-repairmodal-result mt-4 rounded-xl p-3 text-xs flex items-start gap-2 animate-in fade-in-0 duration-150"
      :class="
        resultMessage.type === 'success'
          ? 'border border-emerald-500/20 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
          : 'border border-destructive/20 bg-destructive/10 text-destructive'
      "
    >
      <CheckCircle2 v-if="resultMessage.type === 'success'" class="fe-repairmodal-result-success-icon h-4 w-4 shrink-0 mt-0.5" />
      <X v-else class="fe-repairmodal-result-error-icon h-4 w-4 shrink-0 mt-0.5" />
      <span class="fe-repairmodal-result-text">{{ resultMessage.text }}</span>
    </div>

    <Transition name="fe-tab-fade" mode="out-in">
    <!-- Tab 1: 6-Character Code -->
    <div v-if="activeTab === 'code'" key="code" class="fe-repairmodal-code-tab mt-5 space-y-4">
      <div class="fe-repairmodal-code-group">
        <label for="repair-pairing-code" class="fe-repairmodal-code-label block text-xs font-medium text-foreground mb-1.5">
          Pairing Code (from mobile screen)
        </label>
        <input
          id="repair-pairing-code"
          v-model="codeInput"
          type="text"
          maxlength="32"
          placeholder="e.g. A3F89C"
          autocomplete="off"
          spellcheck="false"
          class="fe-repairmodal-code-input w-full rounded-xl border border-input bg-background px-3.5 py-2.5 text-center font-mono text-lg font-bold uppercase tracking-widest text-foreground shadow-xs placeholder:text-muted-foreground/40 placeholder:tracking-normal focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          @keyup.enter="() => submitRepair(codeInput)"
        />
        <p class="fe-repairmodal-code-hint mt-1.5 text-[11px] text-muted-foreground">
          Enter the 6-character code (or 32-character session ID) currently showing on your mobile screen.
        </p>
      </div>

      <button
        type="button"
        class="fe-repairmodal-submit w-full inline-flex min-h-[44px] items-center justify-center gap-2 rounded-xl bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground shadow-sm transition hover:bg-primary/90 active:scale-[0.98] disabled:opacity-50"
        :disabled="loading || !codeInput.trim()"
        @click="() => submitRepair(codeInput)"
      >
        <Loader2 v-if="loading" class="fe-repairmodal-submit-loading-icon h-4 w-4 animate-spin" />
        <RefreshCw v-else class="fe-repairmodal-submit-icon h-4 w-4" />
        <span class="fe-repairmodal-submit-label">{{ loading ? "Re-pairing..." : "Re-pair Device" }}</span>
      </button>
    </div>

    <!-- Tab 2: Camera Scanner -->
    <DeviceCameraScanner
      v-else
      key="camera"
      ref="scannerRef"
      :active="open && activeTab === 'camera'"
      @scan="(code) => submitRepair(code)"
    />
    </Transition>
  </AppDialog>
</template>

<style scoped>
/* Tab switch: same tokens as mobile stock-symbol-flip (opacity + 6px rise) */
.fe-tab-fade-enter-active,
.fe-tab-fade-leave-active {
  transition: opacity 0.16s ease-out,
    transform 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}
.fe-tab-fade-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.fe-tab-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@media (prefers-reduced-motion: reduce) {
  .fe-tab-fade-enter-active,
  .fe-tab-fade-leave-active {
    transition: opacity 0.2s ease !important;
    transform: none !important;
  }
}
</style>

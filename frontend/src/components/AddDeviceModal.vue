<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch, useTemplateRef } from "vue";
import { Camera, CheckCircle2, KeyRound, Smartphone, X } from "@lucide/vue";
import { useAuth } from "../composables/useAuth";
import DeviceCodeForm from "./DeviceCodeForm.vue";
import DeviceCameraScanner from "./DeviceCameraScanner.vue";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const { authorizeDevice } = useAuth();

const activeTab = ref<"camera" | "code">("code");
const loading = ref(false);
const resultMessage = ref<{ type: "success" | "error"; text: string } | null>(null);

const codeFormRef = useTemplateRef<InstanceType<typeof DeviceCodeForm>>("codeFormRef");
const scannerRef = useTemplateRef<InstanceType<typeof DeviceCameraScanner>>("scannerRef");
const dialogRef = useTemplateRef<HTMLDivElement>("dialogRef");
let closeTimer: ReturnType<typeof setTimeout> | undefined;
let lastSubmitAt = 0;

function clearCloseTimer(): void {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = undefined;
  }
}

async function submitAuthorization(code: string, deviceName: string = "Mobile Device"): Promise<void> {
  const trimmed = code.trim();
  if (!trimmed) {
    resultMessage.value = { type: "error", text: "Please enter the pairing code from your mobile device." };
    return;
  }
  // extractSessionOrCode returns "" for non-conforming QR payloads
  // (foreign URLs, wrong schemes, bad shapes): never send those.
  if (!/^[A-Za-z0-9]{6,32}$/.test(trimmed)) {
    resultMessage.value = { type: "error", text: "That QR code is not a valid pairing code. Please scan the code on your mobile device." };
    return;
  }
  // Client-side 1/sec throttle mirrors the server limiter and prevents
  // scanner bursts from hammering authorize.
  const now = Date.now();
  if (loading.value || now - lastSubmitAt < 1000) return;
  lastSubmitAt = now;

  loading.value = true;
  resultMessage.value = null;

  const res = await authorizeDevice(trimmed, deviceName || "Mobile Device");
  loading.value = false;

  if (res.success) {
    resultMessage.value = { type: "success", text: res.message };
    codeFormRef.value?.reset();
    clearCloseTimer();
    closeTimer = setTimeout(() => {
      emit("close");
    }, 2000);
  } else {
    resultMessage.value = { type: "error", text: res.message };
  }
}

function handleClose(): void {
  clearCloseTimer();
  void scannerRef.value?.stopCamera();
  resultMessage.value = null;
  codeFormRef.value?.reset();
  emit("close");
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") handleClose();
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      resultMessage.value = null;
      window.addEventListener("keydown", handleKeydown);
      void nextTick(() => dialogRef.value?.focus());
    } else {
      window.removeEventListener("keydown", handleKeydown);
      clearCloseTimer();
      void scannerRef.value?.stopCamera();
    }
  }
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
  clearCloseTimer();
});
</script>

<template>
  <div
    v-if="open"
    class="fe-addmodal-overlay fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in-0 duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="add-device-title"
  >
    <div
      ref="dialogRef"
      tabindex="-1"
      class="fe-addmodal-dialog relative w-full max-w-md rounded-2xl border border-border bg-card p-6 text-card-foreground shadow-2xl outline-none animate-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="fe-addmodal-header flex items-center justify-between border-b border-border/80 pb-4">
        <div class="fe-addmodal-header-left flex items-center gap-2.5">
          <span class="fe-addmodal-header-icon flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-primary">
            <Smartphone class="fe-addmodal-header-glyph h-5 w-5" />
          </span>
          <div class="fe-addmodal-header-text">
            <h2 id="add-device-title" class="fe-addmodal-title text-base font-semibold leading-tight text-foreground">
              Add Mobile Device
            </h2>
            <p class="fe-addmodal-subtitle text-xs text-muted-foreground mt-0.5">Link your mobile frontend with your account</p>
          </div>
        </div>
        <button
          type="button"
          class="fe-addmodal-close-btn rounded-lg p-1.5 text-muted-foreground transition hover:bg-accent hover:text-foreground"
          aria-label="Close"
          @click="handleClose"
        >
          <X class="fe-addmodal-close-icon h-4 w-4" />
        </button>
      </div>

      <!-- Tab Switcher -->
      <div class="fe-addmodal-tabs mt-4 flex rounded-lg bg-muted p-1 text-xs">
        <button
          type="button"
          class="fe-addmodal-tab-code flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium transition"
          :class="activeTab === 'code' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
          @click="activeTab = 'code'"
        >
          <KeyRound class="fe-addmodal-tab-code-icon h-3.5 w-3.5" />
          6-Character Code
        </button>
        <button
          type="button"
          class="fe-addmodal-tab-camera flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium transition"
          :class="activeTab === 'camera' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
          @click="activeTab = 'camera'"
        >
          <Camera class="fe-addmodal-tab-camera-icon h-3.5 w-3.5" />
          Camera Scanner
        </button>
      </div>

      <!-- Result Banner -->
      <div
        v-if="resultMessage"
        class="fe-addmodal-result mt-4 rounded-xl p-3 text-xs flex items-start gap-2 animate-in fade-in-0 duration-150"
        :class="
          resultMessage.type === 'success'
            ? 'border border-emerald-500/20 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
            : 'border border-destructive/20 bg-destructive/10 text-destructive'
        "
      >
        <CheckCircle2 v-if="resultMessage.type === 'success'" class="fe-addmodal-result-success-icon h-4 w-4 shrink-0 mt-0.5" />
        <X v-else class="fe-addmodal-result-error-icon h-4 w-4 shrink-0 mt-0.5" />
        <span class="fe-addmodal-result-text">{{ resultMessage.text }}</span>
      </div>

      <!-- Tab 1: 6-Character Code -->
      <DeviceCodeForm
        v-if="activeTab === 'code'"
        ref="codeFormRef"
        :loading="loading"
        @submit="({ code, deviceName }) => submitAuthorization(code, deviceName)"
      />

      <!-- Tab 2: Camera Scanner -->
      <DeviceCameraScanner
        v-else
        ref="scannerRef"
        :active="open && activeTab === 'camera'"
        @scan="(code) => submitAuthorization(code, 'Mobile Device (QR)')"
      />
    </div>
  </div>
</template>

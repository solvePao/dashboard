<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useTemplateRef, watch } from "vue";
import { Loader2 } from "@lucide/vue";
import { QrScannerManager } from "../services/qrScanner";

const props = defineProps<{
  active: boolean;
}>();

const emit = defineEmits<{
  (e: "scan", code: string): void;
}>();

const cameraStarting = ref(false);
const cameraError = ref<string | null>(null);
const readerEl = useTemplateRef<HTMLDivElement>("readerEl");
let qrManager: QrScannerManager | null = null;

async function startCamera(): Promise<void> {
  if (cameraStarting.value || qrManager?.isScanning()) return;
  await nextTick();
  if (!readerEl.value) return;

  cameraError.value = null;
  cameraStarting.value = true;

  try {
    if (!qrManager) {
      qrManager = new QrScannerManager("qr-reader");
    }

    await qrManager.start(
      async (decodedCode) => {
        await stopCamera();
        emit("scan", decodedCode);
      },
      () => {
        // frame decode failure, ignore
      }
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : "Failed to access webcam";
    cameraError.value = `Camera error: ${message}. You can enter the 6-character code instead.`;
  } finally {
    cameraStarting.value = false;
  }
}

async function stopCamera(): Promise<void> {
  if (qrManager) {
    await qrManager.stop();
    qrManager = null;
  }
}

watch(
  () => props.active,
  (isActive) => {
    if (isActive) {
      void startCamera();
    } else {
      void stopCamera();
    }
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  void stopCamera();
});

defineExpose({
  stopCamera,
});
</script>

<template>
  <div class="fe-scanner-root mt-5 flex flex-col items-center">
    <div
      v-if="cameraStarting"
      class="fe-scanner-starting flex h-56 w-full flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border bg-muted/40 text-xs text-muted-foreground animate-in fade-in-0 duration-150"
    >
      <Loader2 class="fe-scanner-starting-icon h-6 w-6 animate-spin text-primary" />
      <span class="fe-scanner-starting-text">Starting webcam...</span>
    </div>

    <div
      v-if="cameraError"
      class="fe-scanner-error rounded-xl border border-destructive/20 bg-destructive/10 p-3 text-xs text-destructive text-center mb-3 animate-in fade-in-0 duration-150"
    >
      {{ cameraError }}
    </div>

    <div
      id="qr-reader"
      ref="readerEl"
      class="fe-scanner-reader w-full overflow-hidden rounded-xl border border-border bg-black/90 shadow-inner"
      style="min-height: 240px"
    ></div>

    <p class="fe-scanner-hint mt-3 text-center text-xs text-muted-foreground">
      Point your webcam towards the QR code on your mobile screen.
    </p>
  </div>
</template>


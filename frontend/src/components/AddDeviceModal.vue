<script setup lang="ts">
import { ref, watch, useTemplateRef } from "vue";
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

async function submitAuthorization(code: string, deviceName: string = "Mobile Device"): Promise<void> {
  const trimmed = code.trim();
  if (!trimmed) {
    resultMessage.value = { type: "error", text: "Please enter the pairing code from your mobile device." };
    return;
  }

  loading.value = true;
  resultMessage.value = null;

  const res = await authorizeDevice(trimmed, deviceName || "Mobile Device");
  loading.value = false;

  if (res.success) {
    resultMessage.value = { type: "success", text: res.message };
    codeFormRef.value?.reset();
    setTimeout(() => {
      emit("close");
    }, 2000);
  } else {
    resultMessage.value = { type: "error", text: res.message };
  }
}

function handleClose(): void {
  void scannerRef.value?.stopCamera();
  resultMessage.value = null;
  codeFormRef.value?.reset();
  emit("close");
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      resultMessage.value = null;
    } else {
      void scannerRef.value?.stopCamera();
    }
  }
);
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-sm animate-in fade-in-0 duration-200"
    role="dialog"
    aria-modal="true"
    aria-labelledby="add-device-title"
  >
    <div
      class="relative w-full max-w-md rounded-2xl border border-border bg-card p-6 text-card-foreground shadow-2xl animate-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border/80 pb-4">
        <div class="flex items-center gap-2.5">
          <span class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-primary">
            <Smartphone class="h-5 w-5" />
          </span>
          <div>
            <h2 id="add-device-title" class="text-base font-semibold leading-tight text-foreground">
              Add Mobile Device
            </h2>
            <p class="text-xs text-muted-foreground mt-0.5">Link your mobile frontend with your account</p>
          </div>
        </div>
        <button
          type="button"
          class="rounded-lg p-1.5 text-muted-foreground transition hover:bg-accent hover:text-foreground"
          aria-label="Close"
          @click="handleClose"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Tab Switcher -->
      <div class="mt-4 flex rounded-lg bg-muted p-1 text-xs">
        <button
          type="button"
          class="flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium transition"
          :class="activeTab === 'code' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
          @click="activeTab = 'code'"
        >
          <KeyRound class="h-3.5 w-3.5" />
          6-Character Code
        </button>
        <button
          type="button"
          class="flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium transition"
          :class="activeTab === 'camera' ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
          @click="activeTab = 'camera'"
        >
          <Camera class="h-3.5 w-3.5" />
          Camera Scanner
        </button>
      </div>

      <!-- Result Banner -->
      <div
        v-if="resultMessage"
        class="mt-4 rounded-xl p-3 text-xs flex items-start gap-2 animate-in fade-in-0 duration-150"
        :class="
          resultMessage.type === 'success'
            ? 'border border-emerald-500/20 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
            : 'border border-destructive/20 bg-destructive/10 text-destructive'
        "
      >
        <CheckCircle2 v-if="resultMessage.type === 'success'" class="h-4 w-4 shrink-0 mt-0.5" />
        <X v-else class="h-4 w-4 shrink-0 mt-0.5" />
        <span>{{ resultMessage.text }}</span>
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

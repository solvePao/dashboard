<script setup lang="ts">
import { ref } from "vue";
import { Check, Loader2 } from "@lucide/vue";

defineProps<{
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: "submit", payload: { code: string; deviceName: string }): void;
}>();

const codeInput = ref("");
const deviceName = ref("Mobile Device");

function normalizeCode(raw: string): string {
  const trimmed = raw.trim().replace(/\s+/g, "");
  return trimmed.length <= 12 ? trimmed.toUpperCase() : trimmed;
}

function handleSubmit(): void {
  const code = normalizeCode(codeInput.value);
  if (!code) return;
  emit("submit", {
    code,
    deviceName: deviceName.value.trim() || "Mobile Device",
  });
}

function reset(): void {
  codeInput.value = "";
}

defineExpose({
  reset,
});
</script>

<template>
  <div class="mt-5 space-y-4">
    <div>
      <label for="pairing-code" class="block text-xs font-medium text-foreground mb-1.5">
        Pairing Code (from mobile screen)
      </label>
      <input
        id="pairing-code"
        v-model="codeInput"
        type="text"
        maxlength="128"
        placeholder="e.g. A3F89C"
        autocomplete="off"
        spellcheck="false"
        class="w-full rounded-xl border border-input bg-background px-3.5 py-2.5 text-center font-mono text-lg font-bold uppercase tracking-widest text-foreground shadow-xs placeholder:text-muted-foreground/40 placeholder:tracking-normal focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        @keyup.enter="handleSubmit"
      />
      <p class="mt-1.5 text-[11px] text-muted-foreground">
        Enter the 6-character code (or full 128-character session ID) displayed on your mobile screen.
      </p>
    </div>

    <div>
      <label for="device-name" class="block text-xs font-medium text-foreground mb-1.5">
        Device Label
      </label>
      <input
        id="device-name"
        v-model="deviceName"
        type="text"
        placeholder="Mobile Device"
        class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground shadow-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      />
    </div>

    <button
      type="button"
      class="w-full inline-flex items-center justify-center gap-2 rounded-xl bg-primary px-4 py-2.5 text-xs font-semibold text-primary-foreground shadow-sm transition hover:bg-primary/90 disabled:opacity-50"
      :disabled="loading || !codeInput.trim()"
      @click="handleSubmit"
    >
      <Loader2 v-if="loading" class="h-4 w-4 animate-spin" />
      <Check v-else class="h-4 w-4" />
      <span>{{ loading ? "Authorizing..." : "Authorize Device" }}</span>
    </button>
  </div>
</template>


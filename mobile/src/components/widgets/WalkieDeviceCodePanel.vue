<script setup lang="ts">
import { ref } from "vue";
import { X, KeyRound, Copy, Check, RotateCcw } from "@lucide/vue";
import { useWalkieDeviceCode } from "../../composables/useWalkieDeviceCode";

const props = defineProps<{
  open: boolean;
  unassigned?: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const { activeCode, expiry, busy, error, message, generate, cancel, copy } = useWalkieDeviceCode(() => props.open);
const copied = ref(false);

async function handleCopy(): Promise<void> {
  await copy();
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}
</script>

<template>
  <transition name="sheet-backdrop">
    <div
      v-if="open"
      class="fixed inset-0 z-40 bg-black/60 backdrop-blur-xs sm:absolute sm:z-20 sm:bg-black/50 sm:backdrop-blur-[2px] flex flex-col justify-end"
      @click.self="emit('close')"
    >
      <transition name="sheet-content" appear>
        <div
          class="relative z-10 mx-auto w-full max-w-lg rounded-t-2xl border-t border-border bg-card p-3.5 sm:p-5 pb-5 sm:pb-8 flex flex-col gap-2.5 sm:gap-4 text-card-foreground max-h-[92dvh] overflow-y-auto shadow-2xl"
          role="dialog"
          aria-modal="true"
          aria-labelledby="device-code-title"
        >
          <!-- Grab handle for mobile touch feel -->
          <div class="mx-auto h-1 w-12 rounded-full bg-muted-foreground/30 mb-0.5 sm:mb-1" />

          <!-- Header -->
          <div class="flex items-center justify-between gap-2 border-b border-border/60 pb-2.5 sm:pb-3">
            <div class="flex items-center gap-2.5">
              <div class="flex h-7 w-7 sm:h-8 sm:w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <KeyRound class="h-3.5 w-3.5 sm:h-4 sm:w-4" />
              </div>
              <div>
                <h3 id="device-code-title" class="text-xs sm:text-sm font-semibold">Add device to group</h3>
                <p class="text-[10px] sm:text-[11px] text-muted-foreground">Share code with group owner</p>
              </div>
            </div>
            <button
              type="button"
              class="rounded-full p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
              aria-label="Close"
              @click="emit('close')"
            >
              <X class="h-4 w-4" />
            </button>
          </div>

          <!-- Code display card -->
          <div v-if="activeCode" class="relative overflow-hidden rounded-xl border border-primary/25 bg-muted/40 p-2.5 sm:p-4 text-center">
            <p class="text-[10px] sm:text-[11px] font-medium uppercase tracking-wider text-muted-foreground">6-digit pairing code</p>
            <p class="mt-0.5 sm:mt-1 font-mono text-2xl sm:text-3xl font-bold tracking-[0.25em] text-foreground select-all" aria-label="Device code">
              {{ activeCode.code }}
            </p>
            <div class="mt-1.5 sm:mt-2 flex items-center justify-center gap-2 text-[10px] sm:text-[11px] text-muted-foreground">
              <span class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-500" />
              <span>Single use · expires at {{ expiry }}</span>
            </div>
          </div>

          <!-- Helper text when no code is active -->
          <div v-else class="rounded-xl border border-dashed border-border p-3.5 sm:p-5 text-center">
            <KeyRound class="mx-auto h-7 w-7 sm:h-8 sm:w-8 text-muted-foreground/50 mb-1.5 sm:mb-2" />
            <p class="text-xs font-medium text-foreground">No active code</p>
            <p class="mt-1 text-[10px] sm:text-[11px] text-muted-foreground leading-relaxed">
              Generate a 10-minute code to let a group owner admit this device to their walkie-talkie group.
            </p>
          </div>

          <!-- Alert messages -->
          <p v-if="error" role="alert" class="rounded-lg border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive">
            {{ error }}
          </p>
          <p v-if="message" role="status" class="rounded-lg border border-border bg-muted/50 p-2 text-xs text-muted-foreground">
            {{ message }}
          </p>

          <!-- Actions -->
          <div class="flex flex-col gap-2 pt-0.5 sm:pt-1">
            <div class="flex gap-2">
              <button
                type="button"
                :disabled="busy"
                class="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-primary px-3 sm:px-4 py-2 sm:py-2.5 text-xs font-semibold text-primary-foreground shadow-sm hover:bg-primary/90 disabled:opacity-50 transition active:scale-95"
                @click="generate"
              >
                <RotateCcw v-if="activeCode" class="h-3.5 w-3.5" />
                <span>{{ activeCode ? 'Generate new code' : 'Generate device code' }}</span>
              </button>

              <button
                v-if="activeCode"
                type="button"
                :disabled="busy"
                class="inline-flex items-center justify-center gap-1.5 rounded-xl border border-border bg-card px-3 sm:px-4 py-2 sm:py-2.5 text-xs font-medium hover:bg-muted disabled:opacity-50 transition active:scale-95"
                @click="handleCopy"
              >
                <Check v-if="copied" class="h-3.5 w-3.5 text-emerald-500" />
                <Copy v-else class="h-3.5 w-3.5" />
                <span>{{ copied ? 'Copied' : 'Copy' }}</span>
              </button>
            </div>

            <button
              v-if="activeCode"
              type="button"
              :disabled="busy"
              class="rounded-xl py-1.5 sm:py-2 text-center text-xs text-muted-foreground hover:text-destructive disabled:opacity-50 transition"
              @click="cancel"
            >
              Cancel active code
            </button>
          </div>

          <!-- Footnote -->
          <p class="text-center text-[9px] sm:text-[10px] leading-relaxed text-muted-foreground">
            Generating a code does not transmit audio. Your microphone stays off until you explicitly enable it.
          </p>
        </div>
      </transition>
    </div>
  </transition>
</template>

<style scoped>
.sheet-backdrop-enter-active,
.sheet-backdrop-leave-active {
  transition: opacity 0.25s ease;
}

.sheet-backdrop-enter-from,
.sheet-backdrop-leave-to {
  opacity: 0;
}

.sheet-content-enter-active {
  transition: transform 0.28s cubic-bezier(0.16, 1, 0.3, 1);
}

.sheet-content-leave-active {
  transition: transform 0.22s ease-in;
}

.sheet-content-enter-from,
.sheet-content-leave-to {
  transform: translateY(100%);
}
</style>

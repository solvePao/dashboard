<script setup lang="ts">
import type { AuthState } from "../types/auth";

defineProps<{
  authState: AuthState;
  statusMessage: string;
}>();
</script>

<template>
  <div class="mo-steps-root flex flex-col gap-2.5">
    <p class="mo-steps-intro text-xs font-medium text-muted-foreground">
      Follow these steps to connect this device:
    </p>

    <ol class="mo-steps-list space-y-2 text-xs">
      <li class="mo-steps-item-1 flex items-start gap-2">
        <span
          class="mo-steps-num-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          1
        </span>
        <span class="mo-steps-text-1 text-foreground/90">
          Open the web dashboard on your computer and sign in with Google.
        </span>
      </li>
      <li class="mo-steps-item-2 flex items-start gap-2">
        <span
          class="mo-steps-num-2 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          2
        </span>
        <span class="mo-steps-text-2 text-foreground/90">
          Click <strong class="mo-steps-text-2-strong font-semibold text-foreground">Add Device</strong> in the top navigation bar.
        </span>
      </li>
      <li class="mo-steps-item-3 flex items-start gap-2">
        <span
          class="mo-steps-num-3 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          3
        </span>
        <span class="mo-steps-text-3 text-foreground/90">
          Scan the QR code or enter the 6-character code from the right panel.
        </span>
      </li>
      <li class="mo-steps-item-4 flex items-start gap-2">
        <span
          class="mo-steps-num-4 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          4
        </span>
        <span class="mo-steps-text-4 text-foreground/90">
          Keep this page open — it signs in on its own once authorized. The round button below re-checks manually.
        </span>
      </li>
    </ol>

    <!-- Feedback Alert -->
    <div
      v-if="statusMessage"
      :class="[
        'mo-steps-feedback',
        'rounded-lg p-2.5 text-xs transition-all',
        authState === 'authorized'
          ? 'border border-emerald-500/20 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
          : authState === 'expired'
          ? 'border border-destructive/20 bg-destructive/10 text-destructive'
          : 'border border-border bg-accent/40 text-muted-foreground'
      ]"
    >
      {{ statusMessage }}
    </div>
  </div>
</template>


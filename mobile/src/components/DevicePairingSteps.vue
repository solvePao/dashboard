<script setup lang="ts">
import type { AuthState } from "../types/auth";

defineProps<{
  authState: AuthState;
  statusMessage: string;
}>();
</script>

<template>
  <div class="flex flex-col gap-2.5">
    <p class="text-xs font-medium text-muted-foreground">
      Follow these steps to connect this device:
    </p>

    <ol class="space-y-2 text-xs">
      <li class="flex items-start gap-2">
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          1
        </span>
        <span class="text-foreground/90">
          Open the web dashboard on your computer and sign in with Google.
        </span>
      </li>
      <li class="flex items-start gap-2">
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          2
        </span>
        <span class="text-foreground/90">
          Click <strong class="font-semibold text-foreground">Add Device</strong> in the top navigation bar.
        </span>
      </li>
      <li class="flex items-start gap-2">
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          3
        </span>
        <span class="text-foreground/90">
          Scan the QR code or enter the 6-character code from the right panel.
        </span>
      </li>
      <li class="flex items-start gap-2">
        <span
          class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 font-mono text-[11px] font-semibold text-primary"
        >
          4
        </span>
        <span class="text-foreground/90">
          Click the <strong class="font-semibold text-foreground">Check for Auth</strong> button below.
        </span>
      </li>
    </ol>

    <!-- Feedback Alert -->
    <div
      v-if="statusMessage"
      :class="[
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


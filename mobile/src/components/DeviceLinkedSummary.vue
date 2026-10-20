<script setup lang="ts">
import { CheckCircle2 } from "@lucide/vue";
</script>

<template>
  <div class="mo-linked-root flex flex-col gap-2.5">
    <div class="mo-linked-success flex items-start gap-2.5 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3 text-xs">
      <CheckCircle2 class="mo-linked-success-icon mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
      <div class="mo-linked-success-text min-w-0">
        <p class="mo-linked-success-title font-medium text-emerald-600 dark:text-emerald-400">Pairing Complete</p>
        <p class="mo-linked-success-desc text-muted-foreground mt-0.5">
          This device is successfully authenticated. A secure 6-month token is stored for all future API requests.
        </p>
      </div>
    </div>

    <div class="mo-linked-facts rounded-lg border border-border/80 bg-muted/30 p-3 text-xs space-y-1.5">
      <div class="mo-linked-fact-status-row flex justify-between text-muted-foreground">
        <span class="mo-linked-fact-status-key">Status:</span>
        <span class="mo-linked-fact-status-value font-medium text-foreground">Active</span>
      </div>
      <div class="mo-linked-fact-validity-row flex justify-between text-muted-foreground">
        <span class="mo-linked-fact-validity-key">Token Validity:</span>
        <span class="mo-linked-fact-validity-value font-medium text-foreground">6 Months</span>
      </div>
      <div class="mo-linked-fact-env-row flex justify-between text-muted-foreground">
        <span class="mo-linked-fact-env-key">Environment:</span>
        <span class="mo-linked-fact-env-value font-medium text-foreground">Mobile Dashboard</span>
      </div>
    </div>
  </div>
</template>


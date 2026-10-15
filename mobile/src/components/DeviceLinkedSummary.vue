<script setup lang="ts">
import { CheckCircle2 } from "@lucide/vue";
</script>

<template>
  <div class="flex flex-col gap-2.5">
    <div class="flex items-start gap-2.5 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3 text-xs">
      <CheckCircle2 class="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
      <div class="min-w-0">
        <p class="font-medium text-emerald-600 dark:text-emerald-400">Pairing Complete</p>
        <p class="text-muted-foreground mt-0.5">
          This device is successfully authenticated. A secure 6-month token is stored for all future API requests.
        </p>
      </div>
    </div>

    <div class="rounded-lg border border-border/80 bg-muted/30 p-3 text-xs space-y-1.5">
      <div class="flex justify-between text-muted-foreground">
        <span>Status:</span>
        <span class="font-medium text-foreground">Active</span>
      </div>
      <div class="flex justify-between text-muted-foreground">
        <span>Token Validity:</span>
        <span class="font-medium text-foreground">6 Months</span>
      </div>
      <div class="flex justify-between text-muted-foreground">
        <span>Environment:</span>
        <span class="font-medium text-foreground">Mobile Dashboard</span>
      </div>
    </div>
  </div>
</template>


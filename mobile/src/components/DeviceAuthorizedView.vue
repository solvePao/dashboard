<script setup lang="ts">
import { ShieldCheck, Sparkles } from "@lucide/vue";
import type { UserProfile } from "../types/auth";

defineProps<{
  user: UserProfile;
}>();
</script>

<template>
  <div
    class="flex flex-1 flex-col items-center justify-center p-4 text-center animate-in fade-in zoom-in-95 duration-300"
  >
    <!-- Profile Photo / Avatar -->
    <div class="relative mb-4">
      <img
        v-if="user.picture_url"
        :src="user.picture_url"
        alt=""
        referrerpolicy="no-referrer"
        class="h-24 w-24 rounded-full border-4 border-primary/20 object-cover shadow-md"
      />
      <div
        v-else
        class="flex h-24 w-24 items-center justify-center rounded-full border-4 border-primary/20 bg-muted text-2xl font-bold text-foreground shadow-md"
      >
        {{ (user.name || user.email || "?").charAt(0).toUpperCase() }}
      </div>
      <span
        class="absolute bottom-1 right-1 flex h-6 w-6 items-center justify-center rounded-full bg-emerald-500 text-white shadow"
        title="Active Device"
      >
        <ShieldCheck class="h-3.5 w-3.5" />
      </span>
    </div>

    <!-- Name & Email -->
    <h2 class="text-xl font-bold tracking-tight text-foreground sm:text-2xl">
      {{ user.name || "Authorized User" }}
    </h2>
    <p class="mt-1 text-sm text-muted-foreground">
      {{ user.email }}
    </p>

    <!-- Badge Info -->
    <div class="mt-4 flex flex-wrap items-center justify-center gap-2">
      <span
        class="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-600 dark:text-emerald-400 border border-emerald-500/20"
      >
        <span class="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
        Connected & Authorized
      </span>
      <span
        class="inline-flex items-center gap-1 rounded-full bg-secondary px-3 py-1 text-xs font-medium text-secondary-foreground"
      >
        <Sparkles class="h-3 w-3 text-amber-500" />
        Valid for 6 Months
      </span>
    </div>

    <p class="mt-4 max-w-xs text-xs text-muted-foreground/80">
      This mobile device is securely linked to your account. Future API requests will use your persistent 6-month token.
    </p>
  </div>
</template>


<script setup lang="ts">
import type { SessionUser } from "../types/auth";

defineProps<{
  user: SessionUser;
}>();
</script>

<template>
  <section
    class="rounded-xl border border-border bg-card p-5 text-card-foreground shadow-sm sm:p-6"
  >
    <div class="flex items-center gap-4">
      <img
        v-if="user.picture_url"
        :src="user.picture_url"
        alt=""
        referrerpolicy="no-referrer"
        class="h-11 w-11 shrink-0 rounded-full object-cover"
      />
      <span
        v-else
        class="flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-muted text-sm font-semibold text-muted-foreground"
        aria-hidden="true"
      >
        {{ (user.name || user.email || "?").charAt(0).toUpperCase() }}
      </span>
      <div class="min-w-0">
        <h1 class="truncate text-base font-semibold leading-tight text-foreground">
          Welcome, {{ user.name || "friend" }}
        </h1>
        <p class="truncate text-sm text-muted-foreground">{{ user.email }}</p>
      </div>
    </div>
  </section>
</template>


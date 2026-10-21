<script setup lang="ts">
import type { SessionUser } from "../types/auth";

defineProps<{
  user: SessionUser;
}>();
</script>

<template>
  <section
    class="fe-profile-card rounded-xl border border-border bg-card p-4 text-card-foreground shadow-sm animate-in fade-in-50 duration-200 sm:rounded-2xl sm:p-6"
  >
    <div class="fe-profile-row flex items-center gap-3.5 sm:gap-4">
      <img
        v-if="user.picture_url"
        :src="user.picture_url"
        alt=""
        referrerpolicy="no-referrer"
        class="fe-profile-avatar-img h-10 w-10 sm:h-11 sm:w-11 shrink-0 rounded-full object-cover"
      />
      <span
        v-else
        class="fe-profile-avatar-fallback flex h-10 w-10 sm:h-11 sm:w-11 shrink-0 items-center justify-center rounded-full bg-muted text-sm font-semibold text-muted-foreground"
        aria-hidden="true"
      >
        {{ (user.name || user.email || "?").charAt(0).toUpperCase() }}
      </span>
      <div class="fe-profile-text min-w-0">
        <h1 class="fe-profile-greeting truncate text-base font-semibold leading-tight text-foreground">
          Welcome, {{ user.name || "friend" }}
        </h1>
        <p class="fe-profile-email truncate text-sm text-muted-foreground">{{ user.email }}</p>
      </div>
    </div>
  </section>
</template>


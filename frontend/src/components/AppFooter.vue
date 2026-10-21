<script setup lang="ts">
const appVersion = import.meta.env.VITE_GIT_COMMIT_HASH || "dev";
</script>

<template>
  <footer class="fe-footer-root py-6 pb-safe text-center">
    <span class="fe-footer-version font-mono text-[11px] text-muted-foreground/60 select-none">
      v({{ appVersion }})
    </span>
  </footer>
</template>


<script setup lang="ts">
import { ref, watch } from "vue";
const props = withDefaults(
  defineProps<{
    name: string;
    pictureUrl?: string | null;
    sizeClass?: string;
  }>(),
  {
    pictureUrl: null,
    sizeClass: "h-7 w-7 text-xs",
  }
);
const failed = ref(false);
watch(() => props.pictureUrl, () => { failed.value = false; });
</script>

<template>
  <span
    :class="[
      'inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/10 font-semibold text-primary select-none',
      sizeClass,
    ]"
  >
    <img v-if="pictureUrl && !failed" :src="pictureUrl" :alt="name" class="h-full w-full object-cover" referrerpolicy="no-referrer" @error="failed = true" />
    <span v-else aria-hidden="true">{{ name.trim().charAt(0).toUpperCase() || '?' }}</span>
  </span>
</template>

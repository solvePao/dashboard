<script setup lang="ts">
import { computed } from "vue";
import type { SplitRatio } from "../types/layout";

export type { SplitRatio };

const props = withDefaults(
  defineProps<{
    ratio?: SplitRatio;
  }>(),
  {
    ratio: "50-50",
  }
);

const leftWidthClass = computed(() => {
  switch (props.ratio) {
    case "30-70":
      return "w-[30%]";
    case "70-30":
      return "w-[70%]";
    case "50-50":
    default:
      return "w-1/2";
  }
});

const rightWidthClass = computed(() => {
  switch (props.ratio) {
    case "30-70":
      return "w-[70%]";
    case "70-30":
      return "w-[30%]";
    case "50-50":
    default:
      return "w-1/2";
  }
});
</script>

<template>
  <div
    class="relative flex h-full w-full flex-row items-stretch gap-2.5 sm:gap-3 md:gap-4 overflow-hidden p-2.5 sm:p-3 md:p-4 pt-[max(0.75rem,env(safe-area-inset-top))] pb-[max(0.75rem,env(safe-area-inset-bottom))] pl-[max(0.75rem,env(safe-area-inset-left))] pr-[max(0.75rem,env(safe-area-inset-right))]"
  >
    <!-- Left Box -->
    <section
      :class="[
        leftWidthClass,
        'group relative flex h-full min-w-0 shrink-0 flex-col overflow-hidden rounded-xl border border-border bg-card text-card-foreground shadow-sm transition-all duration-300 ease-in-out',
      ]"
      aria-label="Left Panel"
    >
      <div class="flex h-full w-full flex-col overflow-y-auto p-3.5 sm:p-4">
        <slot name="left" />
      </div>
    </section>

    <!-- Right Box -->
    <section
      :class="[
        rightWidthClass,
        'group relative flex h-full min-w-0 flex-1 flex-col overflow-hidden rounded-xl border border-border bg-card text-card-foreground shadow-sm transition-all duration-300 ease-in-out',
      ]"
      aria-label="Right Panel"
    >
      <div class="flex h-full w-full flex-col overflow-y-auto p-3.5 sm:p-4">
        <slot name="right" />
      </div>
    </section>
  </div>
</template>

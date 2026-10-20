<script setup lang="ts">
import { watch } from "vue";
import { DialogContent, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";

const props = defineProps<{ open: boolean; title: string }>();
const emit = defineEmits<{ close: [] }>();
let returnFocusTo: HTMLElement | null = null;

watch(() => props.open, (open) => {
  if (open) returnFocusTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
}, { flush: "sync" });

function restoreFocus(event: Event): void {
  if (returnFocusTo?.isConnected) {
    event.preventDefault();
    returnFocusTo.focus();
  }
}
</script>

<template>
  <DialogRoot :open="open" @update:open="(value) => { if (!value) emit('close'); }">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-[60] bg-background/80 backdrop-blur-sm" />
      <DialogContent
        :aria-describedby="undefined"
        @close-auto-focus="restoreFocus"
        class="fe-dialog fixed left-1/2 top-1/2 z-[61] max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 overflow-y-auto overscroll-contain rounded-2xl border border-border bg-card p-5 text-card-foreground shadow-2xl outline-none sm:p-6"
      >
        <DialogTitle as="span" class="sr-only">{{ title }}</DialogTitle>
        <slot />
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

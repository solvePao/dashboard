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
      <DialogOverlay class="fixed inset-0 z-[60] bg-background/80 backdrop-blur-sm data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 duration-200" />
      <DialogContent
        :aria-describedby="undefined"
        @close-auto-focus="restoreFocus"
        class="fe-dialog fixed inset-x-0 bottom-0 z-[61] w-full max-h-[88dvh] overflow-y-auto overscroll-contain rounded-t-2xl rounded-b-none border-t border-border bg-card p-4 pb-safe text-card-foreground shadow-2xl outline-none duration-200 ease-out data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom sm:left-1/2 sm:top-1/2 sm:bottom-auto sm:inset-x-auto sm:w-[calc(100%-2rem)] sm:max-w-md sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-2xl sm:border sm:p-6 sm:data-[state=open]:slide-in-from-bottom-0 sm:data-[state=open]:zoom-in-95 sm:data-[state=closed]:zoom-out-95"
      >
        <div class="mx-auto mb-2.5 h-1 w-10 shrink-0 rounded-full bg-muted-foreground/30 sm:hidden" aria-hidden="true"></div>
        <DialogTitle as="span" class="sr-only">{{ title }}</DialogTitle>
        <slot />
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

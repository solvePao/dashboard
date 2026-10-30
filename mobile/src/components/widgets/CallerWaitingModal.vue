<script setup lang="ts">
import { X } from "@lucide/vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import { useCaller } from "../../composables/useWalkieTalkie";
import CallerInvitations from "./CallerInvitations.vue";

const { waitingModalOpen, pendingCalls, inLiveCall } = useCaller();

function updateOpen(open: boolean): void {
  waitingModalOpen.value = open;
}
</script>

<template>
  <DialogRoot :open="waitingModalOpen && inLiveCall && pendingCalls.length > 0" @update:open="updateOpen">
    <DialogPortal to=".caller-widget-root">
      <DialogOverlay class="absolute inset-0 z-50 bg-black/60" />
      <DialogContent
        class="absolute left-1/2 top-1/2 z-50 flex max-h-[calc(100%_-_1.5rem)] w-[calc(100%_-_1.5rem)] max-w-sm -translate-x-1/2 -translate-y-1/2 flex-col gap-3 overflow-hidden rounded-2xl border border-border bg-card p-4 text-card-foreground shadow-xl focus:outline-none"
        @touchstart.stop
        @touchmove.stop
        @touchend.stop
        @wheel.stop
      >
        <div class="flex shrink-0 items-start justify-between gap-2">
          <div class="min-w-0">
            <DialogTitle class="text-sm font-semibold">Call waiting</DialogTitle>
            <DialogDescription class="mt-1 text-xs leading-relaxed text-muted-foreground">
              Answering leaves your current call. Close to keep talking.
            </DialogDescription>
          </div>
          <DialogClose
            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-border transition hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            aria-label="Close call waiting and keep current call"
          >
            <X class="h-4 w-4" aria-hidden="true" />
          </DialogClose>
        </div>
        <div class="min-h-0 overflow-y-auto overscroll-contain">
          <CallerInvitations :calls="pendingCalls" waiting />
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

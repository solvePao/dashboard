<script setup lang="ts">
import { Phone, PhoneOff } from "@lucide/vue";
import { useCaller, type CallerInvitation } from "../../composables/useWalkieTalkie";
import WalkieParticipantAvatar from "./WalkieParticipantAvatar.vue";

defineProps<{ call: CallerInvitation; actions?: boolean }>();
const { connect, decline, switching, busyElsewhere } = useCaller();
</script>

<template>
  <div v-if="actions" class="flex justify-center gap-6">
    <div class="flex flex-col items-center gap-1">
      <button type="button" :disabled="switching || busyElsewhere" :aria-label="`Decline ${call.name}`" class="caller-round-action flex items-center justify-center rounded-full bg-rose-600 text-white transition hover:bg-rose-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="decline(call)"><PhoneOff class="h-5 w-5" /></button>
      <span class="text-xs font-medium">Decline</span>
    </div>
    <div class="flex flex-col items-center gap-1">
      <button type="button" :disabled="switching || busyElsewhere" :aria-label="`Answer ${call.name}`" class="caller-round-action flex items-center justify-center rounded-full bg-emerald-600 text-white transition hover:bg-emerald-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="connect(call.groupId, call.callId)"><Phone class="h-5 w-5" /></button>
      <span class="text-xs font-medium">Answer</span>
    </div>
  </div>
  <div v-else class="caller-featured-person flex flex-col items-center rounded-3xl bg-primary/5 p-4 text-center [overflow-wrap:anywhere]">
    <p class="caller-featured-status text-xs font-semibold uppercase tracking-wider text-primary">Incoming call</p>
    <WalkieParticipantAvatar :name="call.starter" :picture-url="call.starterPicture" size-class="caller-featured-avatar text-xl" class="ring-4 ring-primary/10" />
    <div class="min-w-0">
      <p class="caller-featured-name text-lg font-semibold">{{ call.starter }}</p>
      <p class="mt-1 text-xs text-muted-foreground">{{ call.name }} · {{ call.participants }} {{ call.participants === 1 ? 'participant' : 'participants' }}</p>
    </div>
  </div>
</template>

<style scoped>
.caller-featured-avatar {
  margin-block: 0.5rem;
  width: 5rem;
  height: 5rem;
}

@container caller (max-width: 340px) or (max-height: 340px) {
  .caller-featured-person {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.375rem 0.625rem;
    padding: 0.625rem;
    text-align: start;
  }

  .caller-featured-status { grid-column: 1 / -1; }
  .caller-featured-avatar { width: 2.5rem; height: 2.5rem; margin: 0; }
  .caller-featured-name { font-size: 0.9375rem; line-height: 1.25; }
}
</style>

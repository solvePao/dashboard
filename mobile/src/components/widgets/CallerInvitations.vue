<script setup lang="ts">
import { Phone, PhoneOff } from "@lucide/vue";
import { useCaller, type CallerInvitation } from "../../composables/useWalkieTalkie";
import WalkieParticipantAvatar from "./WalkieParticipantAvatar.vue";

withDefaults(defineProps<{ calls: CallerInvitation[]; waiting?: boolean; featured?: boolean }>(), { waiting: false, featured: false });
const { connect, decline, switching, busyElsewhere } = useCaller();
</script>

<template>
  <ul class="space-y-2" aria-label="Group calls" @touchstart.stop @touchend.stop @wheel.stop>
    <li v-for="(call, index) in calls" :key="call.callId"
      :class="featured && index === 0 && call.ringing ? 'caller-featured flex flex-col items-center rounded-3xl bg-primary/5 px-4 py-5 text-center' : 'rounded-2xl border border-border bg-background p-3'">
      <template v-if="featured && index === 0 && call.ringing">
        <div class="caller-featured-person flex min-w-0 flex-col items-center">
        <p class="text-xs font-semibold uppercase tracking-wider text-primary">Incoming call</p>
        <WalkieParticipantAvatar :name="call.starter" :picture-url="call.starterPicture" size-class="h-20 w-20 text-2xl" class="my-3 ring-4 ring-primary/10" />
        <p class="break-words text-lg font-semibold">{{ call.starter }}</p>
        <p class="mt-1 text-xs text-muted-foreground">{{ call.name }} · {{ call.participants }} {{ call.participants === 1 ? 'participant' : 'participants' }}</p>
        </div>
        <div class="caller-featured-actions mt-5 flex justify-center gap-10">
          <div class="flex flex-col items-center gap-1.5">
            <button type="button" :disabled="switching || busyElsewhere" :aria-label="`Decline ${call.name}`" class="flex h-14 w-14 items-center justify-center rounded-full bg-rose-600 text-white transition hover:bg-rose-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50" @click="decline(call)"><PhoneOff class="h-5 w-5" /></button>
            <span class="text-xs font-medium">Decline</span>
          </div>
          <div class="flex flex-col items-center gap-1.5">
            <button type="button" :disabled="switching || busyElsewhere" :aria-label="`Answer ${call.name}`" class="flex h-14 w-14 items-center justify-center rounded-full bg-emerald-600 text-white transition hover:bg-emerald-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50" @click="connect(call.groupId, call.callId)"><Phone class="h-5 w-5" /></button>
            <span class="text-xs font-medium">Answer</span>
          </div>
        </div>
      </template>
      <template v-else>
        <div class="flex items-center gap-3">
          <WalkieParticipantAvatar :name="call.starter" :picture-url="call.starterPicture" size-class="h-10 w-10 text-sm" />
          <div class="min-w-0 flex-1">
            <p class="break-words text-sm font-semibold">{{ call.name }}</p>
            <p class="text-xs text-muted-foreground">{{ call.starter }} · {{ call.ringing ? 'Incoming call' : 'Call in progress' }} · {{ call.participants }} {{ call.participants === 1 ? 'participant' : 'participants' }}</p>
          </div>
        </div>
        <div class="mt-3 flex gap-2">
          <button type="button" :disabled="switching || busyElsewhere" :aria-label="`${waiting ? 'Answer and switch to' : call.ringing ? 'Answer' : 'Join'} ${call.name}`" class="min-h-11 flex-1 rounded-full bg-primary px-4 text-xs font-semibold text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="connect(call.groupId, call.callId)">{{ waiting ? 'Answer and switch' : call.ringing ? 'Answer' : 'Join call' }}</button>
          <button v-if="call.ringing" type="button" :disabled="switching || busyElsewhere" :aria-label="`Decline ${call.name}`" class="min-h-11 rounded-full border border-border px-4 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="decline(call)">Decline</button>
        </div>
      </template>
    </li>
  </ul>
</template>

<style scoped>
@media screen and (max-height: 500px), screen and (orientation: portrait) and (max-width: 500px) {
  .caller-featured { flex-direction: row; justify-content: space-around; gap: 0.75rem; padding: 0.75rem; }
  .caller-featured-actions { margin-top: 0; gap: 1rem; }
  .caller-featured-person :deep(.h-20) { width: 3.5rem; height: 3.5rem; margin: 0.5rem 0; }
}
</style>

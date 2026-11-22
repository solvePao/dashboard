<script setup lang="ts">
import { useCaller, type CallerInvitation } from "../../composables/useWalkieTalkie";
import WalkieParticipantAvatar from "./WalkieParticipantAvatar.vue";

defineProps<{ calls: CallerInvitation[]; waiting?: boolean }>();
const { connect, decline, switching, busyElsewhere } = useCaller();
</script>

<template>
  <ul class="space-y-2" aria-label="Group calls">
    <li v-for="call in calls" :key="call.callId" class="rounded-2xl border border-border bg-background p-3">
      <div class="flex items-center gap-3">
        <WalkieParticipantAvatar :name="call.starter" :picture-url="call.starterPicture" size-class="h-10 w-10 text-sm" />
        <div class="min-w-0 flex-1">
          <p class="[overflow-wrap:anywhere] text-sm font-semibold">{{ call.name }}</p>
          <p class="text-xs text-muted-foreground [overflow-wrap:anywhere]">{{ call.starter }} · {{ call.ringing ? 'Incoming call' : 'Call in progress' }} · {{ call.participants }} {{ call.participants === 1 ? 'participant' : 'participants' }}</p>
        </div>
      </div>
      <div class="mt-3 flex flex-wrap gap-2">
        <button type="button" :disabled="switching || busyElsewhere" :aria-label="`${waiting ? 'Answer and switch to' : call.ringing ? 'Answer' : 'Join'} ${call.name}`" class="min-h-11 min-w-0 flex-[2_1_7rem] rounded-full bg-primary px-4 text-xs font-semibold text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="connect(call.groupId, call.callId)">{{ waiting ? 'Answer and switch' : call.ringing ? 'Answer' : 'Join call' }}</button>
        <button v-if="call.ringing" type="button" :disabled="switching || busyElsewhere" :aria-label="`Decline ${call.name}`" class="min-h-11 flex-[1_1_4.5rem] rounded-full border border-border px-4 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="decline(call)">Decline</button>
      </div>
    </li>
  </ul>
</template>

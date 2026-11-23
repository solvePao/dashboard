<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watchPostEffect } from "vue";
import { Phone, PhoneOff, Mic, MicOff, KeyRound, Volume2, Users } from "@lucide/vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import { useCaller } from "../../composables/useWalkieTalkie";
import WalkieAvatarCluster from "./WalkieAvatarCluster.vue";
import WalkieParticipantAvatar from "./WalkieParticipantAvatar.vue";
import CallerInvitations from "./CallerInvitations.vue";
import CallerFeaturedInvitation from "./CallerFeaturedInvitation.vue";

defineProps<{ config: Record<string, unknown> }>();
const WalkieDeviceCodePanel = defineAsyncComponent(() => import("./WalkieDeviceCodePanel.vue"));
const { state, groups, currentGroup, currentGroupId, participants, currentCall, invitations, activityError, switching,
  busyElsewhere, ringBlocked, previousCall, pendingCalls, waitingModalOpen, inLiveCall, canEndForEveryone,
  connect, switchGroup, leave, markInvitationVisible, toggleMute, enableSound, rejoinPrevious } = useCaller();
const codeSheetOpen = ref(false);
const codeSheetRequested = ref(false);
const confirmEndOpen = ref(false);
const ringingCalls = pendingCalls;
const featuredCall = computed(() => !inLiveCall.value ? ringingCalls.value[0] : undefined);
const ongoingCalls = computed(() => invitations.value.filter(call => !call.ringing));
const pickedUp = computed(() => currentCall.value?.established || state.value.status === "talking");
const callStatus = computed(() => {
  if (switching.value || state.value.status === "connecting") return "Connecting call…";
  if (state.value.status === "reconnecting") return "Reconnecting…";
  if (!pickedUp.value) return "Calling group…";
  if (state.value.playbackBlocked) return "Call audio blocked";
  return state.value.audioPlaying ? "In call" : "Connecting audio…";
});
watchPostEffect(() => {
  for (const call of ringingCalls.value) markInvitationVisible(call);
});
function openCodeSheet(): void {
  codeSheetRequested.value = true;
  codeSheetOpen.value = true;
}
function endForEveryone(): void {
  confirmEndOpen.value = false;
  void leave(true);
}
</script>

<template>
  <section class="caller-widget-root relative flex h-full min-h-0 w-full flex-col overflow-hidden bg-card text-card-foreground" aria-label="Caller">
    <header class="flex shrink-0 items-center justify-between gap-2 border-b border-border/70 px-3 py-1.5">
      <div class="flex min-w-0 items-center gap-2.5">
        <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary"><Phone class="h-4 w-4" /></span>
        <div class="min-w-0"><p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Caller</p><h2 class="truncate text-sm font-semibold">{{ currentGroup?.name || 'Group calls' }}</h2></div>
      </div>
      <button type="button" class="flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-border transition hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" aria-label="Add this device to a group" @click="openCodeSheet"><KeyRound class="h-4 w-4" /></button>
    </header>

    <div
      class="caller-stage min-h-0 flex-1"
      :class="{ 'caller-stage--with-actions': inLiveCall || featuredCall }"
    >
      <div data-pager-scroll-area class="min-h-0 min-w-0 overflow-y-auto overscroll-contain">
        <div class="caller-body mx-auto flex min-h-full w-full max-w-md flex-col gap-3 px-4 py-3">
          <p v-if="busyElsewhere" role="status" class="rounded-2xl border border-amber-500/40 bg-amber-500/10 p-3 text-xs">This device is already on a call in another tab.</p>
          <div v-if="activityError || state.status === 'error'" role="alert" class="rounded-2xl border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">{{ activityError || state.message }}</div>
          <button v-if="previousCall" type="button" class="min-h-11 rounded-full border border-primary px-4 text-xs font-semibold text-primary disabled:opacity-50" :disabled="switching || busyElsewhere" @click="rejoinPrevious">Rejoin previous call</button>

          <template v-if="inLiveCall">
            <button v-if="pendingCalls.length" type="button" class="min-h-11 rounded-full border border-primary/30 bg-primary/10 px-4 text-xs font-semibold text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" @click="waitingModalOpen = true">Incoming {{ pendingCalls.length === 1 ? 'call' : 'calls' }} ({{ pendingCalls.length }}) · View</button>
            <div class="caller-summary flex min-w-0 flex-1 flex-col items-center justify-center gap-3 py-2 text-center" role="status" aria-live="polite">
              <div class="max-w-full"><p class="text-xs font-semibold uppercase tracking-wider text-primary">{{ callStatus }}</p><h3 class="caller-name mt-1 font-semibold [overflow-wrap:anywhere]">{{ currentGroup?.name || state.groupName }}</h3></div>
              <div v-if="participants.length" class="caller-participants flex max-w-full flex-wrap justify-center gap-x-4 gap-y-3 py-2" aria-label="Call participants">
                <div v-for="person in participants" :key="person.connection_id" class="flex w-16 flex-col items-center gap-1.5">
                  <WalkieParticipantAvatar :name="person.first_name || person.name || person.device_name" :picture-url="person.picture_url" size-class="caller-participant-avatar text-lg" class="ring-4 ring-primary/10" />
                  <span class="w-full text-xs font-medium [overflow-wrap:anywhere]">{{ person.first_name || person.name || person.device_name }}</span>
                </div>
              </div>
              <div v-else class="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 text-primary"><Users class="h-7 w-7" /></div>
              <p class="text-xs text-muted-foreground">{{ !pickedUp ? 'Waiting for someone to pick up' : state.muted ? 'Your microphone is muted' : 'Your microphone is on' }}</p>
            </div>
          </template>
          <template v-else>
            <p v-if="switching" class="text-center text-xs text-muted-foreground" role="status">Connecting call…</p>
            <template v-if="featuredCall">
              <CallerFeaturedInvitation :call="featuredCall" />
              <CallerInvitations v-if="ringingCalls.length > 1" :calls="ringingCalls.slice(1)" />
            </template>
            <template v-else>
              <nav v-if="groups.length > 1" class="flex shrink-0 gap-2 overflow-x-auto pb-1" aria-label="Caller groups">
                <button v-for="group in groups" :key="group.id" type="button" :disabled="switching || busyElsewhere" :aria-pressed="group.id === currentGroupId" class="min-h-11 max-w-[90%] shrink-0 rounded-full px-4 py-1 text-xs font-medium [overflow-wrap:anywhere] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60" :class="group.id === currentGroupId ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground'" @click="switchGroup(group.id)">{{ group.name }}</button>
              </nav>
              <div v-if="currentGroup" class="caller-summary flex flex-1 flex-col items-center justify-center gap-3 py-3 text-center">
                <p class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Your group</p>
                <h3 class="caller-name max-w-full font-semibold [overflow-wrap:anywhere]">{{ currentGroup.name }}</h3>
                <WalkieAvatarCluster :members="currentGroup.members" />
                <p class="text-xs text-muted-foreground">{{ currentGroup.members.filter(member => member.online).length }} of {{ currentGroup.members.length }} devices online</p>
                <button v-if="!currentCall" type="button" :disabled="switching || busyElsewhere" class="mt-2 flex min-h-12 w-full items-center justify-center gap-2 rounded-full bg-emerald-600 px-4 text-sm font-semibold text-white hover:bg-emerald-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="currentGroupId && connect(currentGroupId)"><Phone class="h-4 w-4" />Call group</button>
              </div>
              <div v-else-if="!groups.length" class="caller-summary flex flex-1 flex-col items-center justify-center gap-2 py-4 text-center">
                <span class="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 text-primary"><Users class="h-7 w-7" /></span>
                <p class="text-base font-semibold">Join a group to call</p><p class="text-xs text-muted-foreground">Share your device code with a group owner.</p>
                <button type="button" class="mt-2 min-h-11 rounded-full bg-primary px-5 text-xs font-semibold text-primary-foreground" @click="openCodeSheet">Get device code</button>
              </div>
            </template>
            <div v-if="ongoingCalls.length" class="space-y-2"><p class="text-xs font-semibold text-muted-foreground">Calls in progress</p><CallerInvitations :calls="ongoingCalls" /></div>
          </template>
          <button v-if="state.playbackBlocked || ringBlocked" type="button" class="flex min-h-11 items-center justify-center gap-2 rounded-full border border-primary/30 px-4 text-xs font-semibold text-primary" @click="enableSound"><Volume2 class="h-4 w-4" />{{ state.playbackBlocked ? 'Enable call audio' : 'Enable ringtone' }}</button>
          <button v-if="inLiveCall && canEndForEveryone" type="button" :disabled="switching" class="min-h-11 self-center rounded-full px-4 text-xs font-semibold text-rose-500 hover:bg-rose-500/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="confirmEndOpen = true">End for everyone</button>
          <p class="caller-footnote mt-auto text-center text-[10px] text-muted-foreground">{{ inLiveCall ? 'You can browse other widgets during your call.' : 'Keep this dashboard open to receive calls.' }}</p>
        </div>
      </div>
      <div v-if="inLiveCall || featuredCall" class="caller-action-bar bg-card">
        <div v-if="inLiveCall" class="flex justify-center gap-6">
          <div class="flex flex-col items-center gap-1">
            <button type="button" class="caller-round-action flex items-center justify-center rounded-full border border-border bg-muted transition hover:bg-muted/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" :aria-pressed="state.muted" :aria-label="state.muted ? 'Unmute microphone' : 'Mute microphone'" @click="toggleMute"><MicOff v-if="state.muted" class="h-5 w-5" /><Mic v-else class="h-5 w-5" /></button>
            <span class="text-xs font-medium">{{ state.muted ? 'Unmute' : 'Mute' }}</span>
          </div>
          <div class="flex flex-col items-center gap-1">
            <button type="button" :disabled="switching" aria-label="Leave call" class="caller-round-action flex items-center justify-center rounded-full bg-rose-600 text-white transition hover:bg-rose-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50" @click="leave()"><PhoneOff class="h-5 w-5" /></button>
            <span class="text-xs font-medium">Leave</span>
          </div>
        </div>
        <CallerFeaturedInvitation v-else-if="featuredCall" :call="featuredCall" actions />
      </div>
    </div>
    <WalkieDeviceCodePanel v-if="codeSheetRequested" :open="codeSheetOpen" :unassigned="!groups.length" @close="codeSheetOpen = false" />
    <DialogRoot v-if="inLiveCall" v-model:open="confirmEndOpen">
      <DialogPortal to=".caller-widget-root">
        <DialogOverlay class="absolute inset-0 z-50 bg-black/60" @touchstart.stop @touchend.stop @wheel.stop />
        <DialogContent class="absolute left-1/2 top-1/2 z-50 w-[calc(100%_-_2rem)] max-h-[calc(100%_-_1rem)] max-w-sm -translate-x-1/2 -translate-y-1/2 overflow-y-auto overscroll-contain rounded-3xl border border-border bg-card p-4 text-card-foreground shadow-xl focus:outline-none"
          @touchstart.stop @touchend.stop @wheel.stop
        >
          <DialogTitle class="text-base font-semibold">End call for everyone?</DialogTitle>
          <DialogDescription class="mt-2 text-sm text-muted-foreground">Everyone will be disconnected from this group call.</DialogDescription>
          <div class="mt-4 flex flex-wrap gap-2"><DialogClose class="min-h-11 flex-[1_1_5rem] rounded-full border border-border px-4 text-sm font-medium">Cancel</DialogClose><button type="button" class="min-h-11 flex-[1_1_5rem] rounded-full bg-rose-600 px-4 text-sm font-semibold text-white" @click="endForEveryone">End call</button></div>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
  </section>
</template>

<style scoped>
.caller-widget-root {
  container: caller / size;
}

.caller-stage {
  --caller-round-size: 3.5rem;
  display: grid;
  grid-template-rows: minmax(0, 1fr);
}

.caller-stage--with-actions {
  grid-template-rows: minmax(0, 1fr) auto;
}

.caller-action-bar {
  border-top: 1px solid var(--border);
  padding: 0.625rem 0.75rem;
}

.caller-name {
  font-size: 1.25rem;
}

@container caller (max-width: 340px) or (max-height: 340px) {
  .caller-stage { --caller-round-size: 2.75rem; }
  .caller-body { gap: 0.5rem; padding: 0.5rem 0.75rem; }
  .caller-summary { gap: 0.375rem; padding-block: 0.25rem; }
  .caller-name { font-size: 1.0625rem; }
  .caller-participants { gap: 0.375rem 0.5rem; padding-block: 0.25rem; }
  .caller-footnote { display: none; }
  .caller-action-bar { padding-block: 0.375rem; }
}

@container caller (min-width: 360px) and (max-height: 340px) {
  .caller-stage--with-actions {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-rows: minmax(0, 1fr);
  }

  .caller-action-bar {
    display: grid;
    align-items: center;
    border-top: 0;
    border-inline-start: 1px solid var(--border);
  }
}

.caller-stage :deep(.caller-round-action),
.caller-participant-avatar {
  width: var(--caller-round-size);
  height: var(--caller-round-size);
}
</style>

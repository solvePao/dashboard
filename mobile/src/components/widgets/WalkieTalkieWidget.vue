<script setup lang="ts">
import { computed, ref } from "vue";
import { Radio, Volume2, Pause, KeyRound, AlertCircle } from "@lucide/vue";
import WalkieAvatarCluster from "./WalkieAvatarCluster.vue";
import WalkiePTTButton from "./WalkiePTTButton.vue";
import WalkieActivityToast from "./WalkieActivityToast.vue";
import WalkieDeviceCodePanel from "./WalkieDeviceCodePanel.vue";
import { useWalkieTalkie } from "../../composables/useWalkieTalkie";

const props = defineProps<{ config: Record<string, unknown> }>();
const groupId = computed(() => typeof props.config.group_id === "string" ? props.config.group_id : null);
const {
  state,
  enable,
  pause,
  press,
  release,
  resumePlayback,
  groups,
  otherActivity,
  switchGroup,
  switching,
  activityError,
} = useWalkieTalkie(groupId);

const codeSheetOpen = ref(false);

const active = computed(() => ["ready", "calling", "talking", "listening", "busy"].includes(state.value.status));
const connecting = computed(() => ["connecting", "reconnecting"].includes(state.value.status));
const currentSpeaker = computed(() => {
  if (!state.value.speaker) return null;
  return state.value.members.find(m => m.device_id === state.value.speaker?.device_id)
    || state.value.peers.find(p => p.connection_id === state.value.speaker?.connection_id);
});
</script>

<template>
  <section
    class="walkie-widget relative flex h-full min-h-0 w-full flex-col overflow-hidden bg-card text-card-foreground select-none"
    aria-label="Walkie-talkie"
  >
    <!-- Header -->
    <header class="flex shrink-0 items-center justify-between gap-2 border-b border-border/70 px-3 py-2">
      <div class="flex min-w-0 items-center gap-2">
        <div class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Radio class="h-3.5 w-3.5" />
          <span
            v-if="active"
            class="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-emerald-500 ring-2 ring-card"
          />
          <span
            v-else-if="connecting"
            class="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-amber-400 ring-2 ring-card animate-pulse"
          />
        </div>
        <div class="min-w-0">
          <p class="text-[9px] font-semibold uppercase tracking-wider text-muted-foreground">Walkie-talkie</p>
          <h2 class="truncate text-xs font-semibold tracking-tight">{{ state.groupName }}</h2>
        </div>
      </div>

      <div class="flex items-center gap-1 shrink-0">
        <!-- Audio pause button -->
        <button
          v-if="active || connecting"
          type="button"
          class="flex h-7 w-7 items-center justify-center rounded-lg border border-border bg-background text-muted-foreground transition hover:bg-muted hover:text-foreground active:scale-95"
          aria-label="Pause walkie-talkie audio"
          title="Pause audio"
          @click="pause()"
        >
          <Pause class="h-3.5 w-3.5" />
        </button>

        <!-- Pairing code modal trigger -->
        <button
          type="button"
          class="flex h-7 w-7 items-center justify-center rounded-lg border border-border bg-background text-muted-foreground transition hover:bg-muted hover:text-foreground active:scale-95"
          aria-label="Device pairing code"
          title="Add this device to a group"
          @click="codeSheetOpen = true"
        >
          <KeyRound class="h-3.5 w-3.5" />
        </button>
      </div>
    </header>

    <!-- Group switcher tabs (if joined multiple groups) -->
    <nav
      v-if="groups.length > 1"
      class="flex shrink-0 gap-1.5 border-b border-border/50 px-3 py-2 overflow-x-auto no-scrollbar bg-muted/20"
      aria-label="Joined groups"
    >
      <button
        v-for="group in groups"
        :key="group.id"
        type="button"
        :disabled="switching || group.id === groupId"
        class="inline-flex shrink-0 items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-medium transition-colors disabled:opacity-60"
        :class="group.id === groupId
          ? 'bg-primary text-primary-foreground shadow-sm'
          : 'border border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground'"
        @click="switchGroup(group.id)"
      >
        <span>{{ group.name }}</span>
        <span
          v-if="group.id === groupId"
          class="inline-block h-1.5 w-1.5 rounded-full bg-primary-foreground/80"
        />
      </button>
    </nav>

    <!-- Notification / Other activity banner -->
    <div v-if="otherActivity.length || activityError" class="shrink-0 px-3 pt-2">
      <WalkieActivityToast
        :activity="otherActivity"
        :switching="switching"
        @switch="switchGroup"
      />
      <div
        v-if="activityError"
        role="alert"
        class="mt-1 flex items-center gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
      >
        <AlertCircle class="h-3.5 w-3.5 shrink-0" />
        <span class="truncate">{{ activityError }}</span>
      </div>
    </div>

    <!-- Main content (zero-scroll compact layout) -->
    <div class="flex flex-1 min-h-0 flex-col items-center justify-between px-3 py-2 overflow-hidden">
      <!-- Top section: Members & Speaker status -->
      <div class="flex flex-col items-center gap-1 w-full shrink-0">
        <template v-if="groupId">
          <!-- Member Avatar Cluster -->
          <WalkieAvatarCluster
            :members="state.members"
            :speaker-id="state.speaker?.device_id"
          />

          <!-- Live Speaker / State Subtitle -->
          <div class="h-4 flex items-center justify-center text-center">
            <span
              v-if="currentSpeaker"
              class="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-500 animate-pulse"
            >
              <span class="h-1.5 w-1.5 rounded-full bg-emerald-500" />
              {{ currentSpeaker.first_name || currentSpeaker.name }} is speaking
            </span>
            <span
              v-else-if="state.members.length"
              class="text-[11px] text-muted-foreground"
            >
              {{ state.members.filter(m => m.online).length }} of {{ state.members.length }} devices online
            </span>
            <span v-else class="text-[11px] text-muted-foreground">
              No members yet
            </span>
          </div>
        </template>

        <!-- No group state -->
        <template v-else>
          <div class="rounded-xl border border-dashed border-border bg-muted/15 p-3 text-center max-w-xs mt-1">
            <p class="text-xs font-semibold text-foreground">No group assigned</p>
            <p class="mt-0.5 text-[11px] text-muted-foreground leading-snug">
              Share your device code with a group owner to join a walkie group.
            </p>
            <button
              type="button"
              class="mt-2 inline-flex items-center gap-1.5 rounded-lg bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition shadow-sm active:scale-95"
              @click="codeSheetOpen = true"
            >
              <KeyRound class="h-3.5 w-3.5" />
              <span>Get device code</span>
            </button>
          </div>
        </template>
      </div>

      <!-- Center: PTT Button -->
      <div class="my-auto py-1 flex flex-col items-center shrink-0">
        <WalkiePTTButton
          :status="state.status"
          :active="active"
          @press="press"
          @release="release"
        />

        <!-- Status message line -->
        <p class="mt-1 max-w-60 text-center text-[11px] text-muted-foreground" role="status" aria-live="polite">
          {{ state.message }}
        </p>

        <!-- Enable Audio Action -->
        <button
          v-if="!active && !connecting && groupId"
          type="button"
          class="mt-2 inline-flex items-center gap-1.5 rounded-lg bg-primary px-3.5 py-1.5 text-xs font-semibold text-primary-foreground shadow-sm transition hover:bg-primary/90 active:scale-95"
          @click="enable()"
        >
          <Volume2 class="h-3.5 w-3.5" />
          <span>Enable audio</span>
        </button>

        <!-- Blocked Playback Action -->
        <button
          v-if="state.playbackBlocked"
          type="button"
          class="mt-2 inline-flex items-center gap-1.5 rounded-lg border border-primary bg-primary/20 px-3.5 py-1.5 text-xs font-semibold text-primary transition hover:bg-primary/30 active:scale-95 animate-pulse"
          @click="resumePlayback"
        >
          <Volume2 class="h-3.5 w-3.5" />
          <span>Incoming audio · Tap to unmute</span>
        </button>
      </div>
    </div>

    <!-- Device code bottom sheet -->
    <WalkieDeviceCodePanel
      :open="codeSheetOpen"
      :unassigned="!groupId"
      @close="codeSheetOpen = false"
    />
  </section>
</template>

<style scoped>
.no-scrollbar::-webkit-scrollbar {
  display: none;
}
.no-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>

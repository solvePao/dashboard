<script setup lang="ts">
import type { WalkieMember } from "../../types/walkieTalkie";
import WalkieParticipantAvatar from "./WalkieParticipantAvatar.vue";

defineProps<{
  members: WalkieMember[];
  speakerId?: string | null;
}>();
</script>

<template>
  <div v-if="members.length" class="flex flex-col items-center gap-1.5" aria-label="Group members">
    <div class="flex items-center justify-center -space-x-2 py-1">
      <div
        v-for="member in members"
        :key="member.device_id"
        :title="`${member.first_name || member.name} · ${member.online ? 'Online' : 'Offline'}`"
        class="relative transition-transform duration-200"
        :class="{ 'scale-110 z-10': speakerId === member.device_id }"
      >
        <!-- Speaking halo/ring -->
        <span
          v-if="speakerId === member.device_id"
          class="absolute -inset-1 rounded-full border-2 border-emerald-400 animate-ping opacity-60"
        />
        <span
          v-if="speakerId === member.device_id"
          class="absolute -inset-0.5 rounded-full ring-2 ring-emerald-500 ring-offset-2 ring-offset-card"
        />

        <div class="rounded-full ring-2 ring-card bg-card overflow-hidden">
          <WalkieParticipantAvatar
            :name="member.first_name || member.name"
            :picture-url="member.picture_url"
            size-class="h-9 w-9 text-xs"
            :class="['transition-opacity duration-200', !member.online ? 'opacity-40' : '']"
          />
        </div>

        <!-- Online status indicator dot -->
        <span
          class="absolute bottom-0 right-0 h-2.5 w-2.5 rounded-full ring-2 ring-card"
          :class="member.online ? 'bg-emerald-500' : 'bg-muted-foreground/40'"
        />
      </div>
    </div>
  </div>
</template>


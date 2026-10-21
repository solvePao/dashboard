import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";
import { WalkieTalkieSession } from "../services/walkieTalkieSession";
import { WalkieTalkieActivity, type GroupActivity } from "../services/walkieTalkieActivity";
import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { useWidgets } from "./useWidgets";
import { subscribeDeviceSocket } from "./useDeviceAuth";
import type { WalkieGroupSummary, WalkieState } from "../types/walkieTalkie";

export function useWalkieTalkie(groupId: Ref<string | null>) {
  const initial = (): WalkieState => ({ status: "activation", message: groupId.value ? "Enable audio on this phone to talk and listen." : "Generate a device code and share it with the group owner.", groupName: "Walkie-talkie", peers: [], members: [], speaker: null, playbackBlocked: false });
  const state = ref<WalkieState>(initial());
  const groups = ref<WalkieGroupSummary[]>([]);
  const activity = ref<GroupActivity[]>([]);
  const activityError = ref("");
  const switching = ref(false);
  const { refreshWidgets } = useWidgets();
  let session: WalkieTalkieSession | undefined;
  let lifecycle = 0;
  let disposed = false;
  let refreshingSelection = false;
  let lastReportedLostGroupId: string | null = null;
  const diagnostic = (event: string) => new WalkieTalkieApi().diagnostic(event, state.value.status);
  const observer = new WalkieTalkieActivity((nextGroups, nextActivity) => {
    groups.value = nextGroups; activity.value = nextActivity;
    if (nextGroups.length) activityError.value = "";
    const current = nextGroups.find(group => group.id === groupId.value);
    if (current) state.value = { ...state.value, groupName: current.name, members: current.members };
  }, (id, snapshot) => {
    if (id !== groupId.value) return;
    state.value = { ...state.value, groupName: snapshot.group_name || state.value.groupName, members: snapshot.members };
  }, message => { activityError.value = message; }, joined => {
    if (!groupId.value || joined.some(group => group.id === groupId.value) || refreshingSelection) {
      lastReportedLostGroupId = null;
      return;
    }
    if (lastReportedLostGroupId === groupId.value) return;
    lastReportedLostGroupId = groupId.value;
    diagnostic("selection_lost"); pause();
    refreshingSelection = true;
    void refreshWidgets().finally(() => { refreshingSelection = false; });
  });
  watch(groupId, id => {
    if (session) diagnostic("group_changed");
    session?.pause();
    const summary = groups.value.find(group => group.id === id);
    state.value = { ...initial(), groupName: summary?.name || "Walkie-talkie", members: summary?.members || [] };
    session = id ? new WalkieTalkieSession(id, next => {
      const current = groups.value.find(group => group.id === id);
      state.value = { ...next,
        groupName: next.groupName === "Walkie-talkie" ? current?.name || next.groupName : next.groupName,
        members: next.members.length ? next.members : current?.members || [],
      };
    }) : undefined;
    if (id && !disposed && document.visibilityState === "visible") {
      void session?.enable().catch(() => undefined);
    }
  }, { immediate: true });
  const release = () => session?.release();
  const pause = () => { lifecycle++; session?.pause(); };
  const visibility = () => {
    if (document.visibilityState === "visible") {
      observer.start();
      if (groupId.value && session && state.value.status === "activation") {
        void session.enable().catch(() => undefined);
      }
    }
  };
  const pagehide = () => { diagnostic("pagehide"); pause(); observer.stop(); };
  async function switchGroup(id: string): Promise<void> {
    if (switching.value || id === groupId.value) return;
    switching.value = true; activityError.value = "";
    const wasEnabled = ["ready", "calling", "talking", "listening", "busy", "connecting", "reconnecting"].includes(state.value.status);
    pause();
    const operation = lifecycle;
    try {
      const result = await new WalkieTalkieApi().activate(id);
      if (operation !== lifecycle || disposed || document.visibilityState !== "visible") return;
      if (groupId.value === id && wasEnabled) { void session?.enable(); }
      await refreshWidgets();
      if (operation !== lifecycle || disposed || document.visibilityState !== "visible") return;
      if (result.cleanup_pending) activityError.value = "Group switched. Previous audio cleanup is retrying.";
    } catch (error) { if (operation !== lifecycle || disposed) return; activityError.value = error instanceof Error ? error.message : "Could not switch groups."; }
    finally { switching.value = false; }
  }

  const unsubscribeDeviceSocket = subscribeDeviceSocket(event => {
    if (event.status === "walkie_activate") {
      if (!event.group_id || event.group_id === groupId.value) {
        if (state.value.status === "activation") {
          state.value = { ...state.value, message: "Incoming call… connecting audio" };
          diagnostic("wake_enable");
          void session?.enable();
        }
      }
    } else if (event.status === "widgets_updated") {
      void observer.sync();
    }
  });

  window.addEventListener("pagehide", pagehide);
  document.addEventListener("visibilitychange", visibility);
  if (document.visibilityState === "visible") observer.start();
  onBeforeUnmount(() => {
    diagnostic("unmount"); disposed = true; pause(); observer.stop();
    unsubscribeDeviceSocket();
    window.removeEventListener("pagehide", pagehide); document.removeEventListener("visibilitychange", visibility);
  });
  return {
    state,
    groups,
    activityError,
    switching,
    otherActivity: computed(() => activity.value.filter(item => item.groupId !== groupId.value)),
    inLiveCall: computed(() => state.value.inLiveCall || false),
    switchGroup,
    enable: (autoPress = false) => { diagnostic("enable"); return session?.enable(autoPress); },
    pause: () => { diagnostic("pause"); pause(); },
    press: () => session?.press(),
    release,
    startLiveCall: () => session?.startLiveCall(),
    endLiveCall: () => session?.endLiveCall(),
    toggleLiveCall: () => session?.toggleLiveCall(),
    resumePlayback: () => session?.resumePlayback()
  };
}

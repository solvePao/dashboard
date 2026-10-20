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
  let resumeTarget: string | null = null;
  let lifecycle = 0;
  let disposed = false;
  let refreshingSelection = false;
  const observer = new WalkieTalkieActivity((nextGroups, nextActivity) => {
    groups.value = nextGroups; activity.value = nextActivity;
    if (nextGroups.length) activityError.value = "";
    const current = nextGroups.find(group => group.id === groupId.value);
    if (current) state.value = { ...state.value, groupName: current.name, members: current.members };
  }, (id, snapshot) => {
    if (id !== groupId.value) return;
    state.value = { ...state.value, groupName: snapshot.group_name || state.value.groupName, members: snapshot.members };
  }, message => { activityError.value = message; }, joined => {
    if (!groupId.value || joined.some(group => group.id === groupId.value) || refreshingSelection) return;
    pause();
    refreshingSelection = true;
    void refreshWidgets().finally(() => { refreshingSelection = false; });
  });
  watch(groupId, id => {
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
    if (resumeTarget === id && !disposed && document.visibilityState === "visible") {
      resumeTarget = null; void session?.enable();
    }
  }, { immediate: true });
  const release = () => session?.release();
  const pause = () => { lifecycle++; resumeTarget = null; session?.pause(); };
  const visibility = () => {
    if (document.visibilityState === "hidden") { pause(); observer.stop(); }
    else observer.start();
  };
  const pagehide = () => { pause(); observer.stop(); };
  async function switchGroup(id: string): Promise<void> {
    if (switching.value || id === groupId.value) return;
    switching.value = true; activityError.value = "";
    const wasEnabled = ["ready", "calling", "talking", "listening", "busy", "connecting", "reconnecting"].includes(state.value.status);
    pause();
    const operation = lifecycle;
    try {
      const result = await new WalkieTalkieApi().activate(id);
      if (operation !== lifecycle || disposed || document.visibilityState !== "visible") return;
      resumeTarget = wasEnabled ? id : null;
      // Push can arrive before this request completes. Handle either ordering.
      if (groupId.value === id) { resumeTarget = null; if (wasEnabled) void session?.enable(); }
      await refreshWidgets();
      if (operation !== lifecycle || disposed || document.visibilityState !== "visible") { resumeTarget = null; return; }
      if (result.cleanup_pending) activityError.value = "Group switched. Previous audio cleanup is retrying.";
    } catch (error) { resumeTarget = null; if (operation !== lifecycle || disposed) return; activityError.value = error instanceof Error ? error.message : "Could not switch groups."; }
    finally { switching.value = false; }
  }

  const unsubscribeDeviceSocket = subscribeDeviceSocket(event => {
    if (event.status === "walkie_activate") {
      if ((!event.group_id || event.group_id === groupId.value) && document.visibilityState === "visible") {
        if (state.value.status === "activation") {
          state.value = { ...state.value, message: "Your group is calling. Enable audio to talk and listen." };
        }
      }
    }
  });

  window.addEventListener("blur", release);
  window.addEventListener("pagehide", pagehide);
  document.addEventListener("visibilitychange", visibility);
  if (document.visibilityState === "visible") observer.start();
  onBeforeUnmount(() => {
    disposed = true; pause(); observer.stop();
    unsubscribeDeviceSocket();
    window.removeEventListener("blur", release); window.removeEventListener("pagehide", pagehide); document.removeEventListener("visibilitychange", visibility);
  });
  return { state, groups, activityError, switching, otherActivity: computed(() => activity.value.filter(item => item.groupId !== groupId.value)), switchGroup,
    enable: () => session?.enable(), pause, press: () => session?.press(), release, resumePlayback: () => session?.resumePlayback() };
}

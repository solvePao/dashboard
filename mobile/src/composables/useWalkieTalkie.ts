import { computed, inject, onBeforeUnmount, provide, ref, watch, type InjectionKey, type Ref } from "vue";
import { WalkieTalkieSession } from "../services/walkieTalkieSession";
import { WalkieTalkieActivity } from "../services/walkieTalkieActivity";
import { CallerRingtone } from "../services/callerRingtone";
import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "../services/liveSocket";
import { useWidgets } from "./useWidgets";
import type { WalkieGroupSummary, WalkieSnapshot, WalkieState } from "../types/walkieTalkie";

export interface CallerInvitation { groupId: string; name: string; callId: string; starter: string; starterPicture: string | null; startedAt: number; ringing: boolean; participants: number }
const callerKey: InjectionKey<ReturnType<typeof useWalkieTalkie>> = Symbol("caller");

/** Created by the persistent pager parent; widgets only consume its state. */
export function useWalkieTalkie(configuredGroup: Ref<string | null>, enabled: Ref<boolean>) {
  const api = new WalkieTalkieApi();
  const { refreshWidgets } = useWidgets();
  const groups = ref<WalkieGroupSummary[]>([]);
  const selectedGroup = ref(configuredGroup.value);
  const deviceId = ref("");
  const snapshots = ref<Record<string, WalkieSnapshot>>({});
  const activityError = ref("");
  const switching = ref(false);
  const now = ref(Date.now());
  const ringBlocked = ref(false);
  const incomingSequence = ref(0);
  const waitingModalOpen = ref(false);
  const previousCall = ref<{ groupId: string; callId: string } | null>(null);
  const dismissed = ref(new Set<string>());
  const seen = new Set<string>();
  const initial = (): WalkieState => ({ status: "ready", message: "Choose a group to call.", groupName: "Caller", peers: [], members: [], playbackBlocked: false, inLiveCall: false, muted: false });
  const state = ref<WalkieState>(initial());
  const activeGroup = ref<string | null>(null);
  const ownership = ref(liveSocket.callerOwner);
  const busyElsewhere = computed(() => ownership.value.busy && ownership.value.owner_id !== liveSocket.gatewaySocketId);
  let session: WalkieTalkieSession | undefined;
  let generation = 0;
  let disposed = false;
  const ringtone = new CallerRingtone(blocked => { ringBlocked.value = blocked; });
  const invitations = computed<CallerInvitation[]>(() => groups.value.flatMap(group => {
    const snapshot = snapshots.value[group.id];
    const call = snapshot?.call;
    if (!call || (activeGroup.value === group.id && state.value.inLiveCall)) return [];
    const ringing = now.value < call.ringing_until && !call.declined.includes(deviceId.value) && !dismissed.value.has(call.call_id) && call.started_by !== deviceId.value;
    const starter = group.members.find(member => member.device_id === call.started_by);
    return [{ groupId: group.id, name: group.name, callId: call.call_id, starter: starter?.first_name || starter?.name || "Group member", starterPicture: starter?.picture_url || null, startedAt: call.started_at, ringing, participants: call.participants.length }];
  }).sort((a, b) => b.startedAt - a.startedAt));
  const pendingCalls = computed(() => invitations.value.filter(call => call.ringing));
  const inLiveCall = computed(() => state.value.inLiveCall === true);
  const currentGroupId = computed(() => activeGroup.value || selectedGroup.value);
  const currentGroup = computed(() => groups.value.find(group => group.id === currentGroupId.value));
  const currentCall = computed(() => currentGroupId.value ? snapshots.value[currentGroupId.value]?.call : null);
  const canEndForEveryone = computed(() => inLiveCall.value && currentCall.value?.started_by === deviceId.value);
  const participants = computed(() => {
    const snapshot = currentGroupId.value ? snapshots.value[currentGroupId.value] : undefined;
    return snapshot?.clients.filter(peer => snapshot.call?.participants.includes(peer.connection_id)) || [];
  });

  const observer = new WalkieTalkieActivity((next, id) => {
    groups.value = next; deviceId.value = id;
    const ids = new Set(next.map(group => group.id));
    snapshots.value = Object.fromEntries(Object.entries(snapshots.value).filter(([key]) => ids.has(key)));
    if (!selectedGroup.value || !ids.has(selectedGroup.value)) selectedGroup.value = next.find(group => group.active)?.id || next[0]?.id || null;
    if (activeGroup.value && !ids.has(activeGroup.value)) { void stopCall().catch(() => undefined); activityError.value = "You no longer belong to this group."; }
  }, (id, snapshot) => { snapshots.value = { ...snapshots.value, [id]: snapshot }; }, message => { activityError.value = message; });

  watch(configuredGroup, id => { if (!inLiveCall.value && !switching.value) selectedGroup.value = id; });
  watch(enabled, value => {
    if (value) observer.start();
    else { generation++; observer.stop(); snapshots.value = {}; groups.value = []; void stopCall().catch(() => undefined); }
  }, { immediate: true });
  watch(pendingCalls, calls => {
    let newCall = false;
    for (const call of calls) if (!seen.has(call.callId)) { seen.add(call.callId); newCall = true; }
    if (newCall && !busyElsewhere.value) {
      if (!inLiveCall.value) incomingSequence.value++;
    }
    if (!calls.length) waitingModalOpen.value = false;
  });
  watch(() => `${inLiveCall.value}:${busyElsewhere.value}:${pendingCalls.value.map(call => call.callId).join(",")}`, () => {
    ringtone.stop();
    if (pendingCalls.value.length && !busyElsewhere.value) ringtone.play(inLiveCall.value);
  });

  const stopDevice = liveSocket.subscribe("device", data => {
    if (data.status === "caller_ownership") ownership.value = { ...liveSocket.callerOwner };
    if (data.status === "widgets_updated") void observer.sync();
  });
  const unlock = () => { void ringtone.unlock(); };
  window.addEventListener("pointerdown", unlock, { once: true });
  const timer = setInterval(() => { now.value = Date.now(); }, 500);
  const pagehide = () => { generation++; ringtone.stop(); observer.stop(); void stopCall().catch(() => undefined); };
  const pageshow = () => { if (enabled.value) observer.start(); };
  window.addEventListener("pagehide", pagehide);
  window.addEventListener("pageshow", pageshow);

  async function stopCall(endForEveryone = false): Promise<void> {
    const old = session;
    try { if (old) await old.leave(endForEveryone); }
    catch (error) { activityError.value = error instanceof Error ? error.message : "Could not leave call."; throw error; }
    finally { if (session === old) { session = undefined; activeGroup.value = null; } }
  }

  async function connect(groupId: string, callId?: string): Promise<void> {
    if (switching.value || busyElsewhere.value || disposed) return;
    switching.value = true; activityError.value = ""; previousCall.value = null;
    if (callId) dismissed.value = new Set([...dismissed.value, callId]);
    const operation = ++generation;
    let stream: MediaStream | undefined;
    const oldCall = activeGroup.value && state.value.callId ? { groupId: activeGroup.value, callId: state.value.callId } : null;
    try {
      void ringtone.unlock();
      stream = await WalkieTalkieSession.microphone();
      if (operation !== generation || disposed) return;
      if (callId) {
        const latest = await api.callState(groupId);
        if (latest.call?.call_id !== callId) throw new Error("That call has already ended. Your current call is unchanged.");
      }
      if (operation !== generation || disposed) return;
      await stopCall();
      if (operation !== generation || disposed) return;
      selectedGroup.value = groupId; activeGroup.value = groupId;
      state.value = { ...initial(), status: "connecting", message: "Connecting call…" };
      const current: WalkieTalkieSession = new WalkieTalkieSession(groupId, next => {
        if (session !== current) return;
        state.value = next;
        if (!next.inLiveCall && ["error", "ready"].includes(next.status)) activeGroup.value = null;
      });
      session = current;
      const captured = stream; stream = undefined;
      await current.connect(captured, callId);
      if (operation !== generation) return;
      for (const invitation of pendingCalls.value) decline(invitation);
      waitingModalOpen.value = false;
    } catch (error) {
      if (operation !== generation || disposed) return;
      activityError.value = error instanceof Error ? error.message : "Could not connect call.";
      if (!state.value.inLiveCall) {
        activeGroup.value = null;
        if (oldCall && snapshots.value[oldCall.groupId]?.call?.call_id === oldCall.callId) previousCall.value = oldCall;
      }
    } finally { stream?.getTracks().forEach(track => track.stop()); switching.value = false; }
  }

  function decline(call: CallerInvitation): void {
    dismissed.value = new Set([...dismissed.value, call.callId]);
    liveSocket.sendWalkie(call.groupId, { type: "call_decline", call_id: call.callId, observe: true, request_id: crypto.randomUUID() });
  }

  async function switchGroup(id: string): Promise<void> {
    if (switching.value || inLiveCall.value || busyElsewhere.value) return;
    switching.value = true; activityError.value = "";
    try { await api.activate(id); selectedGroup.value = id; await refreshWidgets(); }
    catch (error) { activityError.value = error instanceof Error ? error.message : "Could not select group."; }
    finally { switching.value = false; }
  }

  async function leave(endForEveryone = false): Promise<void> {
    if (switching.value) return;
    switching.value = true;
    try { await stopCall(endForEveryone); state.value = { ...initial(), message: "Call ended." }; }
    catch { /* stopCall exposes the error; media is already silenced. */ }
    finally { switching.value = false; }
  }

  onBeforeUnmount(() => {
    disposed = true; generation++; observer.stop(); void stopCall().catch(() => undefined); ringtone.dispose(); clearInterval(timer); stopDevice();
    window.removeEventListener("pointerdown", unlock); window.removeEventListener("pagehide", pagehide); window.removeEventListener("pageshow", pageshow);
  });

  const caller = { state, groups, currentGroup, currentGroupId, participants, currentCall, pendingCalls, invitations, activityError, switching,
    busyElsewhere, ringBlocked, incomingSequence, waitingModalOpen, previousCall, inLiveCall, canEndForEveryone,
    connect, decline, switchGroup, leave, toggleMute: () => session?.toggleMute(),
    enableSound: async () => { await ringtone.unlock(); await session?.resumePlayback(); },
    rejoinPrevious: () => { if (previousCall.value) void connect(previousCall.value.groupId, previousCall.value.callId); },
  };
  provide(callerKey, caller);
  return caller;
}

export function useCaller() {
  const caller = inject(callerKey);
  if (!caller) throw new Error("Caller must be hosted by the persistent device panel.");
  return caller;
}

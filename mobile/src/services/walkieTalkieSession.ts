import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "./liveSocket";
import { mergeSnapshot } from "./walkieTalkieSnapshot";
import { measureCaller } from "./walkieTalkieTiming";
import type { MediaResponse, WalkieSnapshot, WalkieState } from "../types/walkieTalkie";

interface Receiver {
  pc: RTCPeerConnection;
  audio: HTMLAudioElement;
  controller: AbortController;
  playing: boolean;
  blocked: boolean;
}
interface PendingAction { resolve: (data: Record<string, unknown>) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }

/** Media belongs to one explicit call action, never to widget visibility. */
export class WalkieTalkieSession {
  private readonly api = new WalkieTalkieApi();
  private controller = new AbortController();
  private unsubscribeWalkie?: () => void;
  private microphone?: MediaStream;
  private publisher?: RTCPeerConnection;
  private receivers = new Map<string, Receiver>();
  private actions = new Map<string, PendingAction>();
  private iceServers: RTCIceServer[] = [];
  private connectionId = "";
  private callId = "";
  private snapshot?: WalkieSnapshot;
  private generation = 0;
  private joined = false;
  private muted = false;
  private fastSetup = false;
  private measuredFirstPlayback = false;
  private leaving = false;
  private departing?: Promise<void>;
  private readonly startedAt: number;
  private state: WalkieState = { status: "ready", message: "Ready to call", groupName: "Caller", peers: [], members: [], playbackBlocked: false, audioPlaying: false, inLiveCall: false, muted: false };

  readonly groupId: string;
  private readonly changed: (state: WalkieState) => void;
  constructor(groupId: string, startedAt: number, changed: (state: WalkieState) => void) {
    this.groupId = groupId;
    this.startedAt = startedAt;
    this.changed = changed;
  }

  static async microphone(signal: AbortSignal): Promise<MediaStream> {
    if (!navigator.mediaDevices?.getUserMedia) throw new Error("Microphone access requires HTTPS and a supported browser.");
    signal.throwIfAborted();
    return new Promise((resolve, reject) => {
      const abort = () => reject(signal.reason);
      signal.addEventListener("abort", abort, { once: true });
      void navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }, video: false }).then(stream => {
        if (signal.aborted) {
          stream.getTracks().forEach(track => track.stop());
          return;
        }
        stream.getAudioTracks().forEach(track => { track.enabled = false; });
        resolve(stream);
      }, reject).finally(() => signal.removeEventListener("abort", abort));
    });
  }

  private update(patch: Partial<WalkieState>): void {
    const keys = Object.keys(patch) as (keyof WalkieState)[];
    if (keys.every(key => patch[key] === this.state[key])) return;
    this.state = { ...this.state, ...patch }; this.changed(this.state);
  }

  async connect(stream: MediaStream, callId?: string): Promise<void> {
    const generation = ++this.generation;
    const signal = this.controller.signal;
    this.microphone = stream;
    this.update({ status: "connecting", message: "Connecting call…" });
    try {
      stream.getAudioTracks().forEach(track => { track.onended = () => this.failed(new Error("Microphone disconnected. Join again to retry.")); });
      await liveSocket.whenOpen(signal);
      signal.throwIfAborted();
      await new Promise<void>((resolve, reject) => {
        const finish = (error?: Error) => {
          clearTimeout(timer);
          signal.removeEventListener("abort", abort);
          if (error) reject(error);
          else resolve();
        };
        const timer = setTimeout(() => finish(new Error("Call connection timed out.")), 15_000);
        const abort = () => finish(new Error("Call canceled."));
        signal.addEventListener("abort", abort, { once: true });
        this.unsubscribeWalkie = liveSocket.subscribeWalkie(this.groupId, false, envelope => {
          if (generation !== this.generation) return;
          const data = envelope as Record<string, unknown>;
          if (data.type === "connected" && data.observe === false && typeof data.connection_id === "string") {
            this.connectionId = data.connection_id;
            this.fastSetup = data.fast_setup === true;
            if (typeof data.group_name === "string") this.update({ groupName: data.group_name });
            finish();
          } else if (data.type === "error" && !this.connectionId) {
            finish(new Error(String(data.message || "Could not connect call.")));
          } else this.message(data);
        });
      });
      const preparationStarted = performance.now();
      const ice = await this.api.media<{ ice_servers: RTCIceServer[]; session_id?: string }>(
        this.groupId, this.connectionId, this.fastSetup ? "prepare" : "ice", {}, signal);
      if (generation !== this.generation) return;
      if (this.fastSetup && !ice.session_id) throw new Error("Audio service returned no prepared session.");
      measureCaller("media preparation", preparationStarted);
      this.iceServers = ice.ice_servers;
      const pc = this.createPeer(); this.publisher = pc;
      pc.onconnectionstatechange = () => {
        if (generation === this.generation && ["failed", "disconnected"].includes(pc.connectionState)) this.failed(new Error("Call disconnected. Join again to retry."));
      };
      const track = stream.getAudioTracks()[0];
      if (!track) throw new Error("No microphone was found.");
      const transceiver = pc.addTransceiver(track, { direction: "sendonly" });
      await pc.setLocalDescription(await pc.createOffer());
      await this.gather(pc, signal);
      const result = await this.api.media<MediaResponse>(this.groupId, this.connectionId, "publish", {
        session_id: ice.session_id, sdp: pc.localDescription?.sdp, mid: transceiver.mid,
      }, signal);
      if (generation !== this.generation) return;
      if (!result.sessionDescription) throw new Error("Audio service returned no connection answer.");
      await pc.setRemoteDescription(result.sessionDescription);
      await this.connected(pc, signal);
      measureCaller("tap to publisher connected", this.startedAt);
      if (!this.fastSetup) await this.api.media(this.groupId, this.connectionId, "ready", {}, signal);
      if (generation !== this.generation) return;
      const reply = await this.command(callId ? "call_join" : "call_start", callId, this.fastSetup);
      if (generation !== this.generation) return;
      measureCaller("tap to call acknowledgement", this.startedAt);
      this.callId = String(reply.call_id);
      this.joined = true;
      this.update({ inLiveCall: true, callId: this.callId, status: callId ? "talking" : "calling", message: callId ? "In call" : "Calling group…", muted: false });
      this.microphone?.getAudioTracks().forEach(track => { track.enabled = !this.muted; });
    } catch (error) {
      if (generation !== this.generation) throw error;
      await this.close();
      this.update({ status: "error", message: error instanceof Error ? error.message : "Call connection failed." });
      throw error;
    }
  }

  private command(type: string, callId = this.callId, audioReady = false): Promise<Record<string, unknown>> {
    const requestId = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.actions.delete(requestId); reject(new Error("Call action timed out.")); }, 10_000);
      this.actions.set(requestId, { resolve, reject, timer });
      liveSocket.sendWalkie(this.groupId, { type, call_id: callId, request_id: requestId, audio_ready: audioReady });
    });
  }

  private message(data: Record<string, unknown>): void {
    const action = typeof data.request_id === "string" ? this.actions.get(data.request_id) : undefined;
    if (action && data.connection_id === this.connectionId) {
      clearTimeout(action.timer); this.actions.delete(String(data.request_id));
      if (data.type === "error") action.reject(new Error(String(data.message)));
      else action.resolve(data);
    }
    if (data.type === "snapshot") {
      const previous = this.snapshot;
      this.snapshot = mergeSnapshot(previous, data as unknown as WalkieSnapshot);
      if (this.snapshot !== previous) this.applySnapshot();
    }
    else if (data.type === "socket_closed" || (data.type === "ended" && data.connection_id === this.connectionId)) {
      this.failed(new Error(String(data.reason || "Call disconnected. Join again to retry.")));
    }
  }

  private applySnapshot(): void {
    const snapshot = this.snapshot;
    if (!snapshot) return;
    this.update({ groupName: snapshot.group_name || this.state.groupName, peers: snapshot.clients, members: snapshot.members });
    if (!this.joined) return;
    const call = snapshot.call;
    // A pre-ack snapshot can still describe the previous room state. The
    // acknowledgement is delivered before the authoritative new snapshot.
    if (!call || call.call_id !== this.callId || !call.participants.includes(this.connectionId)) {
      void this.close().then(() => this.update({ status: "ready", message: "Call ended." })).catch(error => this.update({ status: "error", message: String(error) }));
      return;
    }
    this.update({ status: call.established ? "talking" : "calling", message: call.established ? "In call" : "Calling group… waiting for pickup", inLiveCall: true, callId: call.call_id });
    this.syncReceivers(call.participants);
  }

  toggleMute(): void {
    this.muted = !this.muted;
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = this.joined && !this.leaving && !this.muted; });
    this.update({ muted: this.muted });
  }

  leave(endForEveryone = false): Promise<void> {
    if (this.departing) return this.departing;
    // Stop local audio as soon as the user leaves.
    this.leaving = true;
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
    this.receivers.forEach(receiver => this.closeReceiver(receiver));
    this.receivers.clear();
    this.departing = this.depart(endForEveryone);
    return this.departing;
  }

  private async depart(endForEveryone: boolean): Promise<void> {
    try {
      if (this.joined) await this.command(endForEveryone ? "call_end" : "call_leave");
    } finally {
      await this.close();
    }
  }

  private syncReceivers(participants: string[]): void {
    if (this.leaving) return;
    const targets = new Set(this.snapshot?.clients
      .filter(peer => peer.published && peer.connection_id !== this.connectionId && participants.includes(peer.connection_id))
      .map(peer => peer.connection_id));
    for (const [id, receiver] of this.receivers) {
      if (!targets.has(id)) {
        // The room retires the matching provider session when the participant leaves.
        this.closeReceiver(receiver);
        this.receivers.delete(id);
      }
    }
    this.updatePlayback();
    try {
      for (const target of targets) {
        if (!this.receivers.has(target)) this.startReceiver(target);
      }
    } catch (error) {
      this.failed(error);
    }
  }

  private createPeer(): RTCPeerConnection {
    return new RTCPeerConnection({ iceServers: this.iceServers });
  }

  private async gather(pc: RTCPeerConnection, signal: AbortSignal): Promise<void> {
    if (pc.iceGatheringState === "complete") return;
    await new Promise<void>((resolve, reject) => {
      let debounceTimer: ReturnType<typeof setTimeout> | undefined;
      const finish = (error?: Error) => {
        clearTimeout(timer);
        clearTimeout(debounceTimer);
        pc.removeEventListener("icegatheringstatechange", check);
        pc.removeEventListener("icecandidate", onCandidate);
        signal.removeEventListener("abort", abort);
        if (error) reject(error); else resolve();
      };
      const check = () => { if (pc.iceGatheringState === "complete") finish(); };
      const onCandidate = (event: RTCPeerConnectionIceEvent) => {
        if (event.candidate) {
          clearTimeout(debounceTimer);
          debounceTimer = setTimeout(() => finish(), 400);
        } else {
          finish();
        }
      };
      const abort = () => finish(new DOMException("Aborted", "AbortError"));
      const timer = setTimeout(() => finish(), 1500);
      pc.addEventListener("icegatheringstatechange", check);
      pc.addEventListener("icecandidate", onCandidate);
      signal.addEventListener("abort", abort, { once: true });
      if (signal.aborted) abort();
      else check();
    });
  }

  private async connected(pc: RTCPeerConnection, signal: AbortSignal): Promise<void> {
    if (pc.connectionState === "connected") return;
    await new Promise<void>((resolve, reject) => {
      const finish = (error?: Error) => {
        clearTimeout(timer); pc.removeEventListener("connectionstatechange", check); signal.removeEventListener("abort", abort);
        if (error) reject(error); else resolve();
      };
      const check = () => {
        if (pc.connectionState === "connected") finish();
        else if (["closed", "failed"].includes(pc.connectionState)) finish(new Error("Audio connection failed."));
      };
      const abort = () => finish(new DOMException("Aborted", "AbortError"));
      const timer = setTimeout(() => finish(new Error("Audio connection timed out.")), 15_000);
      pc.addEventListener("connectionstatechange", check); signal.addEventListener("abort", abort, { once: true });
      if (signal.aborted) abort();
      else check();
    });
  }

  private startReceiver(target: string): void {
    const pc = this.createPeer();
    const audio = new Audio();
    const receiver: Receiver = { pc, audio, controller: new AbortController(), playing: false, blocked: false };
    const signal = receiver.controller.signal;
    this.receivers.set(target, receiver);
    audio.onplaying = () => {
      if (signal.aborted || this.leaving) return;
      receiver.playing = true;
      receiver.blocked = false;
      this.updatePlayback();
      if (!this.measuredFirstPlayback) {
        this.measuredFirstPlayback = true;
        measureCaller("tap to first incoming playback", this.startedAt);
      }
    };
    pc.ontrack = event => {
      if (signal.aborted || this.leaving) return;
      audio.srcObject = new MediaStream([event.track]);
      void this.playReceiver(receiver);
    };
    void this.subscribe(target, receiver);
  }

  private async subscribe(target: string, receiver: Receiver): Promise<void> {
    const { pc } = receiver;
    const signal = receiver.controller.signal;
    try {
      const result = await this.api.media<MediaResponse>(this.groupId, this.connectionId, "subscribe", {
        target, call_id: this.callId,
      }, signal);
      signal.throwIfAborted();
      if (!result.sessionDescription) throw new Error("No incoming audio offer was returned.");
      await pc.setRemoteDescription(result.sessionDescription);
      signal.throwIfAborted();
      const answer = await pc.createAnswer();
      signal.throwIfAborted();
      await pc.setLocalDescription(answer);
      await this.gather(pc, signal);
      signal.throwIfAborted();
      await this.api.media(this.groupId, this.connectionId, "answer", {
        session_id: result.session_id, sdp: pc.localDescription?.sdp,
      }, signal);
      await this.connected(pc, signal);
      signal.throwIfAborted();
      const check = () => {
        if (signal.aborted || !["failed", "disconnected"].includes(pc.connectionState)) return;
        pc.onconnectionstatechange = null;
        void this.receiverFailed(target, receiver, new Error("Incoming audio disconnected. Join again to retry."));
      };
      pc.onconnectionstatechange = check;
      check();
    } catch (error) {
      await this.receiverFailed(target, receiver, error);
    }
  }

  private async receiverFailed(target: string, receiver: Receiver, error: unknown): Promise<void> {
    const signal = receiver.controller.signal;
    if (signal.aborted || this.leaving) return;
    try {
      const latest = await this.api.callState(this.groupId, signal);
      if (signal.aborted || this.leaving) return;
      if (latest.call?.call_id === this.callId && latest.call.participants.includes(target)) {
        this.failed(error);
      } else {
        // Keep the slot until the next snapshot so stale data cannot restart it.
        this.closeReceiver(receiver);
        this.updatePlayback();
      }
    } catch (stateError) {
      if (!signal.aborted && !this.leaving) this.failed(stateError);
    }
  }

  private updatePlayback(): void {
    const receivers = [...this.receivers.values()];
    this.update({ audioPlaying: receivers.some(receiver => receiver.playing),
      playbackBlocked: receivers.some(receiver => receiver.blocked) });
  }

  private async playReceiver(receiver: Receiver): Promise<void> {
    try {
      await receiver.audio.play();
      receiver.blocked = false;
    } catch {
      if (receiver.controller.signal.aborted || this.leaving) return;
      receiver.blocked = true;
    }
    if (!receiver.controller.signal.aborted) this.updatePlayback();
  }

  async resumePlayback(): Promise<void> {
    await Promise.all([...this.receivers.values()]
      .filter(receiver => receiver.audio.srcObject && !receiver.playing)
      .map(receiver => this.playReceiver(receiver)));
  }

  private failed(error: unknown): void {
    const message = error instanceof Error ? error.message : "Call disconnected.";
    void this.close().then(() => this.update({ status: "error", message })).catch(error => this.update({ status: "error", message: `${message} ${String(error)}` }));
  }

  private closeReceiver(receiver: Receiver): void {
    receiver.controller.abort();
    receiver.playing = false;
    receiver.blocked = false;
    receiver.pc.ontrack = null;
    receiver.pc.onconnectionstatechange = null;
    receiver.audio.onplaying = null;
    receiver.pc.close();
    receiver.audio.pause();
    receiver.audio.srcObject = null;
  }

  private closing?: Promise<void>;
  close(): Promise<void> {
    if (this.closing) return this.closing;
    this.joined = false; this.generation++;
    this.controller.abort();
    for (const action of this.actions.values()) { clearTimeout(action.timer); action.reject(new Error("Call closed.")); }
    this.actions.clear();
    if (this.publisher) { this.publisher.onconnectionstatechange = null; this.publisher.close(); this.publisher = undefined; }
    this.microphone?.getTracks().forEach(track => { track.onended = null; track.stop(); }); this.microphone = undefined;
    this.receivers.forEach(receiver => this.closeReceiver(receiver)); this.receivers.clear();
    this.update({ inLiveCall: false, callId: undefined, muted: false, audioPlaying: false, playbackBlocked: false });
    this.closing = new Promise<void>((resolve, reject) => {
      if (!this.unsubscribeWalkie || !liveSocket.isOpen) { this.unsubscribeWalkie?.(); resolve(); return; }
      const timer = setTimeout(() => { stop(); reject(new Error("Could not release the previous call. Reload this page before calling again.")); }, 12_000);
      const stop = liveSocket.subscribe("walkie", data => {
        if ((data.type === "unsubscribed" && data.observe === false) || data.type === "socket_closed") { clearTimeout(timer); stop(); resolve(); }
        else if (data.type === "error" && data.code === "unsubscribe_failed") { clearTimeout(timer); stop(); reject(new Error(String(data.message))); }
      }, this.groupId);
      this.unsubscribeWalkie(); this.unsubscribeWalkie = undefined;
    });
    return this.closing;
  }
}

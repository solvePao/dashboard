import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "./liveSocket";
import type { MediaResponse, WalkieSnapshot, WalkieState } from "../types/walkieTalkie";

interface Receiver { pc: RTCPeerConnection; audio: HTMLAudioElement; sessionId: string }
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
  private queue: Promise<void> = Promise.resolve();
  private generation = 0;
  private ready = false;
  private joined = false;
  private muted = false;
  private state: WalkieState = { status: "ready", message: "Ready to call", groupName: "Caller", peers: [], members: [], playbackBlocked: false, inLiveCall: false, muted: false };

  readonly groupId: string;
  private readonly changed: (state: WalkieState) => void;
  constructor(groupId: string, changed: (state: WalkieState) => void) { this.groupId = groupId; this.changed = changed; }

  static async microphone(): Promise<MediaStream> {
    if (!navigator.mediaDevices?.getUserMedia) throw new Error("Microphone access requires HTTPS and a supported browser.");
    const stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }, video: false });
    stream.getAudioTracks().forEach(track => { track.enabled = false; });
    return stream;
  }

  private update(patch: Partial<WalkieState>): void {
    this.state = { ...this.state, ...patch }; this.changed(this.state);
  }

  async connect(stream: MediaStream, callId?: string): Promise<void> {
    const generation = ++this.generation;
    const signal = this.controller.signal;
    this.microphone = stream;
    this.update({ status: "connecting", message: "Connecting call…" });
    try {
      stream.getAudioTracks().forEach(track => { track.onended = () => this.failed(new Error("Microphone disconnected. Join again to retry.")); });
      await Promise.race([liveSocket.whenOpen(), new Promise<never>((_, reject) => {
        const timer = setTimeout(() => reject(new Error("Dashboard connection is unavailable.")), 10_000);
        signal.addEventListener("abort", () => { clearTimeout(timer); reject(new Error("Call canceled.")); }, { once: true });
      })]);
      if (signal.aborted) return;
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("Call connection timed out.")), 15_000);
        const abort = () => { clearTimeout(timer); reject(new Error("Call canceled.")); };
        signal.addEventListener("abort", abort, { once: true });
        this.unsubscribeWalkie = liveSocket.subscribeWalkie(this.groupId, false, envelope => {
          if (generation !== this.generation) return;
          const data = envelope as Record<string, unknown>;
          if (data.type === "connected" && data.observe === false && typeof data.connection_id === "string") {
            this.connectionId = data.connection_id;
            if (typeof data.group_name === "string") this.update({ groupName: data.group_name });
            clearTimeout(timer); signal.removeEventListener("abort", abort); resolve();
          } else if (data.type === "error" && !this.connectionId) {
            clearTimeout(timer); signal.removeEventListener("abort", abort);
            reject(new Error(String(data.message || "Could not connect call.")));
          } else this.message(data);
        });
      });
      const ice = await this.api.media<{ ice_servers: RTCIceServer[] }>(this.groupId, this.connectionId, "ice", {}, signal);
      if (generation !== this.generation) return;
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
      const result = await this.api.media<MediaResponse>(this.groupId, this.connectionId, "publish", { sdp: pc.localDescription?.sdp, mid: transceiver.mid }, signal);
      if (generation !== this.generation) return;
      if (!result.sessionDescription) throw new Error("Audio service returned no connection answer.");
      await pc.setRemoteDescription(result.sessionDescription);
      await this.connected(pc, signal);
      await this.api.media(this.groupId, this.connectionId, "ready", {}, signal);
      if (generation !== this.generation) return;
      this.ready = true;
      const reply = await this.command(callId ? "call_join" : "call_start", callId);
      if (generation !== this.generation) return;
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

  private command(type: string, callId = this.callId): Promise<Record<string, unknown>> {
    const requestId = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.actions.delete(requestId); reject(new Error("Call action timed out.")); }, 10_000);
      this.actions.set(requestId, { resolve, reject, timer });
      liveSocket.sendWalkie(this.groupId, { type, call_id: callId, request_id: requestId });
    });
  }

  private message(data: Record<string, unknown>): void {
    const action = typeof data.request_id === "string" ? this.actions.get(data.request_id) : undefined;
    if (action && data.connection_id === this.connectionId) {
      clearTimeout(action.timer); this.actions.delete(String(data.request_id));
      if (data.type === "error") action.reject(new Error(String(data.message)));
      else action.resolve(data);
    }
    if (data.type === "snapshot") { this.snapshot = data as unknown as WalkieSnapshot; this.applySnapshot(); }
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
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = !this.muted; });
    this.applyPlaybackGate();
    this.scheduleSubscriptions(this.generation);
  }

  toggleMute(): void {
    this.muted = !this.muted;
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = this.joined && !this.muted; });
    this.update({ muted: this.muted });
  }

  async leave(endForEveryone = false): Promise<void> {
    // Silence capture at the gesture, without waiting for server I/O.
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
    try { if (this.joined) await this.command(endForEveryone ? "call_end" : "call_leave"); }
    finally { await this.close(); }
  }

  private scheduleSubscriptions(generation: number): void {
    if (!this.ready || !this.joined) return;
    this.queue = this.queue.then(async () => {
      if (generation !== this.generation) return;
      const call = this.snapshot?.call;
      const targets = new Set(this.snapshot?.clients.filter(peer => peer.published && peer.connection_id !== this.connectionId && call?.participants.includes(peer.connection_id)).map(peer => peer.connection_id));
      for (const [id, receiver] of this.receivers) {
        if (!targets.has(id)) { this.closeReceiver(receiver); this.receivers.delete(id); await this.api.media(this.groupId, this.connectionId, "close", { target: id }, this.controller.signal); }
      }
      for (const target of targets) {
        if (generation !== this.generation) return;
        if (!this.receivers.has(target)) {
          try { await this.subscribe(target, generation); }
          catch (error) {
            if (generation !== this.generation) return;
            const latest = await this.api.callState(this.groupId, this.controller.signal);
            if (latest.call?.call_id !== this.callId || !latest.call.participants.includes(target)) {
              const receiver = this.receivers.get(target);
              if (receiver) this.closeReceiver(receiver);
              this.receivers.delete(target);
            } else throw error;
          }
        }
      }
    }).catch(error => { if (generation === this.generation) this.failed(error); });
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
    });
  }

  private async subscribe(target: string, generation: number): Promise<void> {
    const pc = this.createPeer();
    const audio = new Audio(); audio.autoplay = true; audio.muted = true;
    const receiver: Receiver = { pc, audio, sessionId: "" };
    this.receivers.set(target, receiver);
    pc.onconnectionstatechange = () => {
      if (generation === this.generation && this.receivers.get(target) === receiver
          && ["failed", "disconnected"].includes(pc.connectionState)) {
        void this.api.callState(this.groupId, this.controller.signal).then(latest => {
          if (generation === this.generation && latest.call?.call_id === this.callId && latest.call.participants.includes(target)) {
            this.failed(new Error("Incoming audio disconnected. Join again to retry."));
          }
        }).catch(error => { if (generation === this.generation) this.failed(error); });
      }
    };
    pc.ontrack = event => {
      if (generation !== this.generation || this.receivers.get(target) !== receiver) return;
      audio.srcObject = new MediaStream([event.track]);
      this.applyPlaybackGate();
      void audio.play().catch(() => this.update({ playbackBlocked: true }));
    };
    const result = await this.api.media<MediaResponse>(this.groupId, this.connectionId, "subscribe", { target }, this.controller.signal);
    if (generation !== this.generation) return;
    receiver.sessionId = result.session_id;
    if (!result.sessionDescription) throw new Error("No incoming audio offer was returned.");
    await pc.setRemoteDescription(result.sessionDescription);
    if (generation !== this.generation) return;
    await pc.setLocalDescription(await pc.createAnswer());
    if (generation !== this.generation) return;
    await this.gather(pc, this.controller.signal);
    if (generation !== this.generation) return;
    await this.api.media(this.groupId, this.connectionId, "answer", { session_id: receiver.sessionId, sdp: pc.localDescription?.sdp }, this.controller.signal);
    if (generation !== this.generation) return;
    await this.connected(pc, this.controller.signal);

  }

  private applyPlaybackGate(): void {
    const call = this.snapshot?.call;
    for (const [id, receiver] of this.receivers) {
      receiver.audio.muted = !this.joined || !call?.participants.includes(id);
      if (!receiver.audio.muted) void receiver.audio.play().catch(() => this.update({ playbackBlocked: true }));
    }
  }

  async resumePlayback(): Promise<void> {
    try { await Promise.all([...this.receivers.values()].map(receiver => receiver.audio.play())); this.update({ playbackBlocked: false }); }
    catch { this.update({ playbackBlocked: true }); }
  }

  private failed(error: unknown): void {
    const message = error instanceof Error ? error.message : "Call disconnected.";
    void this.close().then(() => this.update({ status: "error", message })).catch(error => this.update({ status: "error", message: `${message} ${String(error)}` }));
  }

  private closeReceiver(receiver: Receiver): void {
    receiver.pc.ontrack = null; receiver.pc.onconnectionstatechange = null;
    receiver.pc.close(); receiver.audio.pause(); receiver.audio.srcObject = null;
  }

  private closing?: Promise<void>;
  close(): Promise<void> {
    if (this.closing) return this.closing;
    this.joined = false; this.ready = false; this.generation++;
    this.controller.abort();
    for (const action of this.actions.values()) { clearTimeout(action.timer); action.reject(new Error("Call closed.")); }
    this.actions.clear();
    if (this.publisher) { this.publisher.onconnectionstatechange = null; this.publisher.close(); this.publisher = undefined; }
    this.microphone?.getTracks().forEach(track => { track.onended = null; track.stop(); }); this.microphone = undefined;
    this.receivers.forEach(receiver => this.closeReceiver(receiver)); this.receivers.clear();
    this.update({ inLiveCall: false, callId: undefined, muted: false });
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

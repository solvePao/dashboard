import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "./liveSocket";
import type { MediaResponse, WalkieSnapshot, WalkieState } from "../types/walkieTalkie";

const armedKey = (groupId: string) => `walkie-armed:${groupId}`;

interface Receiver { pc: RTCPeerConnection; audio: HTMLAudioElement; sessionId: string; disconnectTimer?: ReturnType<typeof setTimeout> }
interface ReceiverRetry { attempts: number; cleanup: boolean; timer?: ReturnType<typeof setTimeout> }

/** Owns one locally activated session. No media or credentials are persisted. */
export class WalkieTalkieSession {
  private readonly api = new WalkieTalkieApi();
  private controller = new AbortController();
  private unsubscribeWalkie?: () => void;
  private microphone?: MediaStream;
  private publisher?: RTCPeerConnection;
  private receivers = new Map<string, Receiver>();
  private receiverRetries = new Map<string, ReceiverRetry>();
  private publisherDisconnectTimer?: ReturnType<typeof setTimeout>;
  private stableTimer?: ReturnType<typeof setTimeout>;
  private iceServers: RTCIceServer[] = [];
  private connectionId = "";
  private snapshot?: WalkieSnapshot;
  private queue: Promise<void> = Promise.resolve();
  private renew?: ReturnType<typeof setInterval>;
  private leaseTimer?: ReturnType<typeof setTimeout>;
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private retry = 0;
  private generation = 0;
  private held = false;
  private calling = false;
  private callTimer?: ReturnType<typeof setTimeout>;
  private requestId = "";
  private leaseId = "";
  private ready = false;
  private enabled = false;
  private pendingPress = false;
  private liveCall = false;
  private pendingLiveCall = false;
  private pendingCallJoin = false;
  private inCall = false;
  private state: WalkieState = { status: "activation", message: "Enable audio on this phone to talk and listen.", groupName: "Walkie-talkie", peers: [], members: [], speaker: null, playbackBlocked: false, inLiveCall: false };

  private readonly groupId: string;
  private readonly changed: (state: WalkieState) => void;
  constructor(groupId: string, changed: (state: WalkieState) => void) {
    this.groupId = groupId;
    this.changed = changed;
  }

  private update(patch: Partial<WalkieState>): void {
    this.state = { ...this.state, ...patch };
    this.changed(this.state);
  }

  async enable(autoPress = false): Promise<void> {
    // pause() clears pending intents; a live-call start/join queued for this
    // fresh start must survive it, or Live pressed from idle does nothing.
    const keepLiveCall = this.pendingLiveCall;
    const keepCallJoin = this.pendingCallJoin;
    this.pause(false);
    this.pendingLiveCall = keepLiveCall;
    this.pendingCallJoin = keepCallJoin;
    this.liveCall = keepLiveCall || this.liveCall;
    this.enabled = true;
    this.retry = 0;
    this.pendingPress = autoPress;
    if (autoPress) {
      this.held = true;
      this.requestId = crypto.randomUUID();
    }
    await this.start(false);
  }

  private async start(reconnecting: boolean): Promise<void> {
    const generation = ++this.generation;
    this.controller = new AbortController();
    const signal = this.controller.signal;
    this.update({ status: reconnecting ? "reconnecting" : "connecting", message: "Connecting audio…", playbackBlocked: false });
    try {
      this.update({ message: "Allow microphone access in your browser to connect…" });
      if (!navigator.mediaDevices?.getUserMedia) throw new Error("Microphone access requires HTTPS and a supported browser.");
      const stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }, video: false });
      stream.getAudioTracks().forEach(track => { track.enabled = false; });
      if (generation !== this.generation) { stream.getTracks().forEach(track => track.stop()); return; }
      this.microphone = stream;
      stream.getAudioTracks().forEach(track => { track.onended = () => this.failed(new Error("Microphone was disconnected. Enable audio again."), false); });
      // Join this group on the phone's single mux socket (channel `walkie`).
      await liveSocket.whenOpen();
      if (generation !== this.generation || signal.aborted) return;
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("Audio connection timed out.")), 15_000);
        const abort = () => { clearTimeout(timer); reject(new DOMException("Aborted", "AbortError")); };
        signal.addEventListener("abort", abort, { once: true });
        this.unsubscribeWalkie = liveSocket.subscribeWalkie(this.groupId, false, (envelope) => {
          if (generation !== this.generation) return;
          const data = envelope as unknown as Record<string, unknown>;
          if (data.type === "socket_closed") {
            if (this.enabled) this.failed(new Error("Connection lost."), true);
            return;
          }
          // Only the media subscription may claim connection_id. The observe
          // subscription for this same group shares the socket and its
          // `connected` reply carries a different, media-less connection.
          if (data.type === "connected" && data.observe === false && typeof data.connection_id === "string") {
            this.connectionId = data.connection_id;
            if (typeof data.group_name === "string") this.update({ groupName: data.group_name });
            clearTimeout(timer); signal.removeEventListener("abort", abort); resolve();
          } else if (data.type === "error") {
            clearTimeout(timer); signal.removeEventListener("abort", abort);
            reject(new Error(typeof data.message === "string" ? data.message : "Audio connection failed."));
          } else { this.message(data); }
        });
      });
      try { localStorage.setItem(armedKey(this.groupId), "1"); } catch { /* private mode */ }
      const ice = await this.api.media<{ ice_servers: RTCIceServer[] }>(this.connectionId, "ice", {}, signal);
      if (generation !== this.generation || signal.aborted) return;
      this.iceServers = ice.ice_servers;
      const pc = this.createPeer();
      this.publisher = pc;
      pc.onconnectionstatechange = () => {
        if (generation !== this.generation) return;
        clearTimeout(this.publisherDisconnectTimer);
        if (pc.connectionState === "failed") this.failed(new Error("Audio network connection interrupted."), true);
        else if (pc.connectionState === "disconnected") {
          this.publisherDisconnectTimer = setTimeout(() => {
            if (generation === this.generation && pc.connectionState === "disconnected") this.failed(new Error("Audio network connection interrupted."), true);
          }, 8000);
        }
      };
      const track = stream.getAudioTracks()[0];
      if (!track) throw new Error("No microphone was found.");
      const transceiver = pc.addTransceiver(track, { direction: "sendonly" });
      await pc.setLocalDescription(await pc.createOffer());
      await this.gather(pc, signal);
      const result = await this.api.media<MediaResponse>(this.connectionId, "publish", { sdp: pc.localDescription?.sdp, mid: transceiver.mid }, signal);
      if (generation !== this.generation || signal.aborted) return;
      if (!result.sessionDescription) throw new Error("Audio service returned no connection answer.");
      await pc.setRemoteDescription(result.sessionDescription);
      await this.connected(pc, signal);
      await this.api.media(this.connectionId, "ready", {}, signal);
      if (generation !== this.generation) return;
      this.ready = true;
      this.stableTimer = setTimeout(() => {
        if (generation === this.generation && this.ready && this.publisher?.connectionState === "connected") this.retry = 0;
      }, 30_000);
      this.refreshStatus();
      this.scheduleSubscriptions(generation);
      // A snapshot may have arrived before `connected` (empty connectionId);
      // re-evaluate any ringing call now that this session can act on it.
      if (this.snapshot?.call) this.handleCallSnapshot();
      if (this.pendingLiveCall) {
        this.pendingLiveCall = false;
        this.send({ type: "call_start", request_id: this.requestId });
      } else if (this.pendingCallJoin) {
        this.pendingCallJoin = false;
        this.send({ type: "call_join" });
      } else if (this.pendingPress && this.held) {
        this.pendingPress = false;
        this.press();
      }
    } catch (error) {
      if (generation !== this.generation || signal.aborted) return;
      const denied = error instanceof DOMException && ["NotAllowedError", "NotFoundError", "NotReadableError"].includes(error.name);
      this.failed(error, reconnecting && !denied);
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

  private message(data: Record<string, unknown>): void {
    if (data.type === "snapshot") {
      this.snapshot = data as unknown as WalkieSnapshot;
      this.update({ groupName: this.snapshot.group_name || this.state.groupName, peers: this.snapshot.clients, members: this.snapshot.members, speaker: this.snapshot.speaker });
      if (this.leaseId && this.snapshot.speaker?.connection_id !== this.connectionId) this.release(true);

      // Server holds a ring window for solo PTT and grants on peer join;
      // no client re-press is needed (it would read as busy against itself).
      this.handleCallSnapshot();

      this.applyPlaybackGate();
      this.refreshStatus();
      this.scheduleSubscriptions(this.generation);
    } else if (data.type === "granted" && typeof data.lease_id === "string") {
      if (!this.held || data.request_id !== this.requestId || !this.ready) {
        this.send({ type: "release", lease_id: data.lease_id }); return;
      }
      this.leaseId = data.lease_id;
      this.armLease();
      this.microphone?.getAudioTracks().forEach(track => { track.enabled = true; });
      clearInterval(this.renew);
      this.renew = setInterval(() => this.send({ type: "renew", lease_id: this.leaseId }), 2000);
      this.refreshStatus();
    } else if (data.type === "renewed" && data.lease_id === this.leaseId) {
      this.armLease();
    } else if (data.type === "calling") {
      this.calling = true;
      clearTimeout(this.callTimer);
      this.callTimer = setTimeout(() => {
        if (this.calling) {
          this.calling = false;
          this.release(true);
          this.update({ status: "ready", message: "No one is available right now." });
        }
      }, 30_000);
      this.refreshStatus();
    } else if (data.type === "call_busy") {
      this.liveCall = false;
      this.pendingLiveCall = false;
      this.update({ status: "busy", message: "There is already a live call in this group.", inLiveCall: false, callState: null });
    } else if (data.type === "call_ended") {
      this.inCall = false;
      this.liveCall = false;
      this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
      this.update({ status: "ready", message: "Live call ended.", inLiveCall: false, callState: null });
      this.refreshStatus();
    } else if (data.type === "busy") {
      this.liveCall = false;
      this.release(true);
      this.update({ status: "busy", message: "Channel busy. Release and press again when your friend finishes.", inLiveCall: false, callState: null });
    } else if (data.type === "ended") {
      this.liveCall = false;
      this.pause(false);
      this.update({ status: "activation", message: typeof data.reason === "string" ? data.reason : "Session ended. Enable audio again.", inLiveCall: false });
    }
  }

  private get armed(): boolean {
    try { return localStorage.getItem(armedKey(this.groupId)) === "1"; } catch { return false; }
  }

  /** React to the room's live-call state carried on every snapshot. */
  private handleCallSnapshot(): void {
    const call = this.snapshot?.call ?? null;
    if (!call) {
      if (this.inCall) {
        this.inCall = false;
        this.liveCall = false;
        this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
        this.update({ inLiveCall: false, callState: null, message: "Live call ended." });
      }
      return;
    }
    const participant = call.participants.includes(this.connectionId);
    if (participant) {
      this.liveCall = true;
      if (call.participants.length >= 2) {
        this.inCall = true;
        // Open mic for the duration of the call; no hold, no lease.
        this.microphone?.getAudioTracks().forEach(track => { track.enabled = true; });
        this.update({ inLiveCall: true, callState: "in-call" });
      } else {
        this.update({ inLiveCall: true, callState: "ringing", status: "ringing", message: "Calling group… waiting for someone to join" });
      }
      return;
    }
    // Incoming call while connected but not yet a participant.
    if (this.inCall) return;
    this.update({ callState: "ringing", status: "ringing", message: "Incoming live call…", inLiveCall: false });
    if (this.armed) {
      // Armed devices enable audio automatically when a call starts.
      // Never re-enter enable() while a start is already in flight: it
      // pauses first and would wipe the pending join.
      if (this.ready) this.send({ type: "call_join" });
      else { this.pendingCallJoin = true; if (!this.enabled) void this.enable(); }
    }
  }

  /** Join a ringing live call (used by the widget's Join tap when unarmed). */
  joinLiveCall(): void {
    if (this.ready) this.send({ type: "call_join" });
    else { this.pendingCallJoin = true; if (!this.enabled) void this.enable(); }
  }

  private armLease(): void {
    clearTimeout(this.leaseTimer);
    // Fail muted before the server's six-second lease expires if acknowledgements stop.
    this.leaseTimer = setTimeout(() => { this.release(true); this.update({ message: "Talk connection interrupted. Press again." }); }, 5000);
  }

  press(): void {
    if (this.held) return;
    this.held = true;
    this.requestId = crypto.randomUUID();

    if (!this.ready) {
      this.pendingPress = true;
      if (!this.enabled) {
        void this.enable(true);
      }
      return;
    }

    // The room wakes the group and holds a 30s ring window server-side;
    // it replies `calling` (no peers yet) or `granted` (floor is yours).
    this.send({ type: "press", request_id: this.requestId });
  }

  release(_force = false): void {
    // A live call is ended only via endLiveCall; PTT release must not drop it.
    void _force;
    if (this.inCall) return;
    this.liveCall = false;
    this.pendingPress = false;
    this.calling = false;
    clearTimeout(this.callTimer);
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
    this.held = false;
    clearInterval(this.renew); clearTimeout(this.leaseTimer);
    if (this.leaseId) this.send({ type: "release", lease_id: this.leaseId });
    this.leaseId = "";
    this.refreshStatus();
  }

  startLiveCall(): void {
    if (this.liveCall) return;
    this.liveCall = true;
    this.requestId = crypto.randomUUID();
    this.update({ callState: "ringing", status: "ringing", message: "Calling group…", inLiveCall: true });
    if (!this.ready) {
      this.pendingLiveCall = true;
      if (!this.enabled) void this.enable();
      return;
    }
    this.send({ type: "call_start", request_id: this.requestId });
  }

  endLiveCall(): void {
    this.send({ type: "call_end" });
    this.inCall = false;
    this.liveCall = false;
    this.pendingLiveCall = false;
    this.pendingCallJoin = false;
    this.microphone?.getAudioTracks().forEach(track => { track.enabled = false; });
    this.update({ inLiveCall: false, callState: null, status: "ready", message: "Live call ended." });
    this.refreshStatus();
  }

  toggleLiveCall(): void {
    // Ringing incoming call from someone else: tap means Join, not End.
    if (this.state.callState === "ringing" && !this.liveCall) {
      this.joinLiveCall();
    } else if (this.liveCall) {
      this.endLiveCall();
    } else {
      this.startLiveCall();
    }
  }

  private send(value: Record<string, unknown>): void {
    liveSocket.sendWalkie(this.groupId, value);
  }

  private refreshStatus(): void {
    if (!this.ready) return;
    if (this.inCall) {
      this.update({ status: "talking", message: "In live call · Tap End to hang up", inLiveCall: true, callState: "in-call" });
      return;
    }
    if (this.snapshot?.call && !this.snapshot.call.participants.includes(this.connectionId)) {
      this.update({ status: "ringing", message: "Incoming live call…", inLiveCall: false, callState: "ringing" });
      return;
    }
    if (this.liveCall) {
      this.update({ status: "ringing", message: "Calling group… waiting for someone to join", inLiveCall: true, callState: "ringing" });
      return;
    }
    if (this.calling) {
      this.update({ status: "calling", message: "Calling group… waiting for peers to join", inLiveCall: false, callState: null });
      return;
    }
    const speaker = this.snapshot?.speaker;
    if (this.leaseId) {
      this.update({
        status: "talking",
        message: "Talking live · release to listen",
        inLiveCall: false,
        callState: null,
      });
    } else if (speaker && speaker.connection_id !== this.connectionId) {
      const name = this.snapshot?.clients.find(peer => peer.connection_id === speaker.connection_id)?.name || "Your friend";
      this.update({ status: "listening", message: `${name} is talking`, inLiveCall: false, callState: null });
    } else {
      this.update({ status: "ready", message: "Channel ready · hold to talk or tap Live", inLiveCall: false, callState: null });
    }
  }

  private hasPublication(target: string): boolean {
    return Boolean(this.snapshot?.clients.some(peer => peer.connection_id === target && peer.published));
  }

  private retryReceiver(target: string, generation: number): void {
    if (generation !== this.generation || !this.ready || !this.hasPublication(target)) return;
    const receiver = this.receivers.get(target);
    if (receiver) this.closeReceiver(receiver);
    this.receivers.delete(target);
    const retry = this.receiverRetries.get(target) || { attempts: 0, cleanup: true };
    this.receiverRetries.set(target, retry);
    retry.cleanup = true;
    if (retry.timer) return;
    if (retry.attempts >= 5) {
      retry.attempts = 6;
      this.update({ message: "Incoming audio could not reconnect. Pause and enable audio to try again." });
      return;
    }
    retry.attempts++;
    this.update({ message: "Incoming audio interrupted. Reconnecting your friend…" });
    retry.timer = setTimeout(() => {
      retry.timer = undefined;
      if (generation === this.generation && this.hasPublication(target)) this.scheduleSubscriptions(generation);
    }, Math.min(1000 * 2 ** (retry.attempts - 1), 15_000));
  }

  private scheduleSubscriptions(generation: number): void {
    if (!this.ready) return;
    this.queue = this.queue.then(async () => {
      if (generation !== this.generation || !this.ready) return;
      const targets = new Set(this.snapshot?.clients.filter(peer => peer.published && peer.connection_id !== this.connectionId).map(peer => peer.connection_id));
      for (const [id, receiver] of this.receivers) {
        if (!targets.has(id)) { this.closeReceiver(receiver); this.receivers.delete(id); }
      }
      for (const [id, retry] of this.receiverRetries) {
        if (!targets.has(id)) { clearTimeout(retry.timer); this.receiverRetries.delete(id); }
      }
      for (const target of targets) {
        if (generation !== this.generation) return;
        const retry = this.receiverRetries.get(target);
        if (this.receivers.has(target) || retry?.timer || (retry && retry.attempts > 5)) continue;
        try {
          if (retry?.cleanup) {
            await this.api.media(this.connectionId, "close", { target }, this.controller.signal);
            if (generation !== this.generation || !this.hasPublication(target)) return;
            retry.cleanup = false;
          }
          await this.subscribe(target, generation);
        } catch {
          if (generation === this.generation && !this.controller.signal.aborted) this.retryReceiver(target, generation);
        }
      }
    }).catch(() => undefined);
  }

  private async subscribe(target: string, generation: number): Promise<void> {
    const pc = this.createPeer();
    const audio = new Audio(); audio.autoplay = true; audio.muted = true;
    const receiver: Receiver = { pc, audio, sessionId: "" };
    this.receivers.set(target, receiver);
    pc.onconnectionstatechange = () => {
      if (generation !== this.generation || this.receivers.get(target) !== receiver) return;
      clearTimeout(receiver.disconnectTimer);
      if (pc.connectionState === "failed") this.retryReceiver(target, generation);
      else if (pc.connectionState === "disconnected") {
        receiver.disconnectTimer = setTimeout(() => {
          if (generation === this.generation && this.receivers.get(target) === receiver && pc.connectionState === "disconnected") this.retryReceiver(target, generation);
        }, 8000);
      }
    };
    pc.ontrack = event => {
      if (generation !== this.generation || this.receivers.get(target) !== receiver) return;
      audio.srcObject = new MediaStream([event.track]);
      this.applyPlaybackGate();
      void audio.play().catch(() => this.update({ playbackBlocked: true }));
    };
    const result = await this.api.media<MediaResponse>(this.connectionId, "subscribe", { target }, this.controller.signal);
    if (generation !== this.generation) return;
    receiver.sessionId = result.session_id;
    if (!result.sessionDescription) throw new Error("No incoming audio offer was returned.");
    await pc.setRemoteDescription(result.sessionDescription);
    if (generation !== this.generation) return;
    await pc.setLocalDescription(await pc.createAnswer());
    if (generation !== this.generation) return;
    await this.gather(pc, this.controller.signal);
    if (generation !== this.generation) return;
    await this.api.media(this.connectionId, "answer", { session_id: receiver.sessionId, sdp: pc.localDescription?.sdp }, this.controller.signal);
    if (generation !== this.generation) return;
    await this.connected(pc, this.controller.signal);
    if (generation === this.generation) this.refreshStatus();
  }

  private applyPlaybackGate(): void {
    const call = this.snapshot?.call;
    const inCall = Boolean(call && call.participants.includes(this.connectionId) && call.participants.length >= 2);
    for (const [id, receiver] of this.receivers) {
      // Live call: hear every other participant. PTT: hear only the speaker.
      const muted = inCall ? id === this.connectionId : this.snapshot?.speaker?.connection_id !== id;
      const wasMuted = receiver.audio.muted;
      receiver.audio.muted = muted;
      if (wasMuted && !muted) void receiver.audio.play().catch(() => this.update({ playbackBlocked: true }));
    }
  }

  async resumePlayback(): Promise<void> {
    try {
      await Promise.all([...this.receivers.values()].map(receiver => receiver.audio.play()));
      this.update({ playbackBlocked: false });
    } catch { this.update({ playbackBlocked: true, message: "Tap enable sound again to allow incoming audio." }); }
  }

  private failed(error: unknown, reconnect: boolean): void {
    const retry = this.retry;
    this.pause(false);
    const message = error instanceof Error ? error.message : "Audio connection failed.";
    if (reconnect && retry < 5 && document.visibilityState === "visible") {
      this.enabled = true; this.retry = retry + 1;
      this.update({ status: "reconnecting", message: "Connection interrupted. Reconnecting muted…" });
      this.reconnectTimer = setTimeout(() => { if (this.enabled) void this.start(true); }, Math.min(1000 * 2 ** retry, 15_000));
    } else this.update({ status: "error", message });
  }

  private closeReceiver(receiver: Receiver): void {
    clearTimeout(receiver.disconnectTimer);
    receiver.pc.ontrack = null;
    receiver.pc.onconnectionstatechange = null;
    receiver.pc.close(); receiver.audio.pause(); receiver.audio.srcObject = null;
  }

  pause(showMessage = true): void {
    this.liveCall = false;
    this.inCall = false;
    this.pendingLiveCall = false;
    this.pendingCallJoin = false;
    this.pendingPress = false;
    this.calling = false;
    clearTimeout(this.callTimer);
    this.release(true);
    this.enabled = false; this.ready = false; this.generation++;
    clearTimeout(this.reconnectTimer);
    clearTimeout(this.publisherDisconnectTimer); clearTimeout(this.stableTimer);
    this.receiverRetries.forEach(retry => clearTimeout(retry.timer)); this.receiverRetries.clear();
    this.controller.abort();
    // Leave this group on the shared socket; the socket itself stays open.
    this.unsubscribeWalkie?.(); this.unsubscribeWalkie = undefined;
    if (this.publisher) { this.publisher.onconnectionstatechange = null; this.publisher.close(); this.publisher = undefined; }
    this.microphone?.getTracks().forEach(track => { track.onended = null; track.stop(); }); this.microphone = undefined;
    this.receivers.forEach(receiver => this.closeReceiver(receiver)); this.receivers.clear();
    this.connectionId = ""; this.snapshot = undefined; this.queue = Promise.resolve();
    this.update({ peers: [], speaker: null, playbackBlocked: false, inLiveCall: false, callState: null, ...(showMessage ? { status: "activation" as const, message: "Audio paused. Enable audio to listen and talk again." } : {}) });
  }
}

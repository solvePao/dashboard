import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import type { MediaResponse, WalkieSnapshot, WalkieState } from "../types/walkieTalkie";

interface Receiver { pc: RTCPeerConnection; audio: HTMLAudioElement; sessionId: string; disconnectTimer?: ReturnType<typeof setTimeout> }
interface ReceiverRetry { attempts: number; cleanup: boolean; timer?: ReturnType<typeof setTimeout> }

/** Owns one locally activated session. No media or credentials are persisted. */
export class WalkieTalkieSession {
  private readonly api = new WalkieTalkieApi();
  private controller = new AbortController();
  private socket?: WebSocket;
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
  private heartbeat?: ReturnType<typeof setInterval>;
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
  private state: WalkieState = { status: "activation", message: "Enable audio on this phone to talk and listen.", groupName: "Walkie-talkie", peers: [], members: [], speaker: null, playbackBlocked: false };

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
    this.pause(false);
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
      const availability = await this.api.availability(signal);
      if (generation !== this.generation || signal.aborted) return;
      this.update({ groupName: availability.group_name, message: "Allow microphone access in your browser to connect…" });
      if (!navigator.mediaDevices?.getUserMedia) throw new Error("Microphone access requires HTTPS and a supported browser.");
      const stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }, video: false });
      stream.getAudioTracks().forEach(track => { track.enabled = false; });
      if (generation !== this.generation) { stream.getTracks().forEach(track => track.stop()); return; }
      this.microphone = stream;
      stream.getAudioTracks().forEach(track => { track.onended = () => this.failed(new Error("Microphone was disconnected. Enable audio again."), false); });
      const ticket = await this.api.connect(signal);
      if (generation !== this.generation || signal.aborted) return;
      if (ticket.group_id !== this.groupId) throw new Error("The assigned group changed. Wait for the dashboard update.");
      this.update({ groupName: ticket.group_name });
      const ws = this.api.socket(ticket.group_id, ticket.ticket);
      this.socket = ws;
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("Audio connection timed out.")), 15_000);
        const abort = () => { clearTimeout(timer); reject(new DOMException("Aborted", "AbortError")); };
        signal.addEventListener("abort", abort, { once: true });
        ws.onmessage = event => {
          if (generation !== this.generation) return;
          let data: Record<string, unknown>;
          try { data = JSON.parse(String(event.data)) as Record<string, unknown>; } catch { return; }
          if (data.type === "connected" && typeof data.connection_id === "string") {
            this.connectionId = data.connection_id;
            clearTimeout(timer); signal.removeEventListener("abort", abort); resolve();
          } else { this.message(data); }
        };
        ws.onerror = () => { clearTimeout(timer); reject(new Error("Audio connection failed.")); };
        ws.onclose = () => {
          clearTimeout(timer);
          reject(new Error("Audio connection closed."));
          if (generation === this.generation && this.enabled) this.failed(new Error("Connection lost."), true);
        };
      });
      this.heartbeat = setInterval(() => this.send({ type: "heartbeat" }), 10_000);
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
      if (this.pendingPress && this.held) {
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
      const finish = (error?: Error) => {
        clearTimeout(timer); pc.removeEventListener("icegatheringstatechange", check); signal.removeEventListener("abort", abort);
        if (error) reject(error); else resolve();
      };
      const check = () => { if (pc.iceGatheringState === "complete") finish(); };
      const abort = () => finish(new DOMException("Aborted", "AbortError"));
      const timer = setTimeout(() => finish(new Error("Could not establish audio network candidates.")), 12_000);
      pc.addEventListener("icegatheringstatechange", check); signal.addEventListener("abort", abort, { once: true });
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
      if (this.leaseId && this.snapshot.speaker?.connection_id !== this.connectionId) this.release();

      if (this.calling && this.held) {
        const peers = (this.snapshot.clients || []).filter(c => c.connection_id !== this.connectionId);
        if (peers.length > 0) {
          this.calling = false;
          clearTimeout(this.callTimer);
          this.send({ type: "press", request_id: this.requestId });
        }
      }

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
    } else if (data.type === "busy") {
      this.release(); this.update({ status: "busy", message: "Channel busy. Release and press again when your friend finishes." });
    } else if (data.type === "ended") {
      this.pause(false);
      this.update({ status: "activation", message: typeof data.reason === "string" ? data.reason : "Session ended. Enable audio again." });
    }
  }

  private armLease(): void {
    clearTimeout(this.leaseTimer);
    // Fail muted before the server's six-second lease expires if acknowledgements stop.
    this.leaseTimer = setTimeout(() => { this.release(); this.update({ message: "Talk connection interrupted. Press again." }); }, 5000);
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

    // Trigger DO wake so all other member devices wake up and connect
    void this.api.wake(this.groupId).catch(() => undefined);

    const peers = (this.snapshot?.clients || []).filter(c => c.connection_id !== this.connectionId);
    if (peers.length === 0) {
      this.calling = true;
      this.update({ status: "calling", message: "Calling group… waiting for peers to join" });
      clearTimeout(this.callTimer);
      this.callTimer = setTimeout(() => {
        if (this.calling) {
          this.calling = false;
          this.release();
          this.update({ status: "ready", message: "No one is available right now." });
        }
      }, 8000);
    } else {
      this.send({ type: "press", request_id: this.requestId });
    }
  }

  release(): void {
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

  private send(value: Record<string, unknown>): void {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(value));
  }

  private refreshStatus(): void {
    if (!this.ready) return;
    if (this.calling) {
      this.update({ status: "calling", message: "Calling group… waiting for peers to join" });
      return;
    }
    const speaker = this.snapshot?.speaker;
    if (this.leaseId) this.update({ status: "talking", message: "Talking live · release to listen" });
    else if (speaker && speaker.connection_id !== this.connectionId) {
      const name = this.snapshot?.clients.find(peer => peer.connection_id === speaker.connection_id)?.name || "Your friend";
      this.update({ status: "listening", message: `${name} is talking` });
    } else this.update({ status: "ready", message: "Channel ready · hold to talk" });
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
    for (const [id, receiver] of this.receivers) {
      const muted = this.snapshot?.speaker?.connection_id !== id;
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
    this.pendingPress = false;
    this.calling = false;
    clearTimeout(this.callTimer);
    this.release();
    this.enabled = false; this.ready = false; this.generation++;
    clearInterval(this.heartbeat); clearTimeout(this.reconnectTimer);
    clearTimeout(this.publisherDisconnectTimer); clearTimeout(this.stableTimer);
    this.receiverRetries.forEach(retry => clearTimeout(retry.timer)); this.receiverRetries.clear();
    this.controller.abort();
    if (this.socket) { this.socket.onclose = null; this.socket.onerror = null; this.socket.onmessage = null; this.socket.close(); this.socket = undefined; }
    if (this.publisher) { this.publisher.onconnectionstatechange = null; this.publisher.close(); this.publisher = undefined; }
    this.microphone?.getTracks().forEach(track => { track.onended = null; track.stop(); }); this.microphone = undefined;
    this.receivers.forEach(receiver => this.closeReceiver(receiver)); this.receivers.clear();
    this.connectionId = ""; this.snapshot = undefined; this.queue = Promise.resolve();
    this.update({ peers: [], speaker: null, playbackBlocked: false, ...(showMessage ? { status: "activation" as const, message: "Audio paused. Enable audio to listen and talk again." } : {}) });
  }
}

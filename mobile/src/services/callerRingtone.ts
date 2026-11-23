import ringtoneUrl from "../assets/universfield-ringtone-030-437513.mp3";

/** Browser audio permission never controls invitations. */
export class CallerRingtone {
  private context?: AudioContext;
  private buffer?: Promise<AudioBuffer>;
  private source?: AudioBufferSourceNode;
  private ringing = false;
  private waiting = false;
  private generation = 0;
  private readonly blocked: (value: boolean) => void;
  constructor(blocked: (value: boolean) => void) { this.blocked = blocked; }

  async unlock(): Promise<void> {
    try {
      this.context ??= new AudioContext();
      const resume = this.context.resume();
      this.blocked(this.context.state !== "running");
      await resume;
      this.blocked(this.context.state !== "running");
      if (this.ringing) void this.tone(this.generation);
    } catch { this.blocked(true); }
  }

  play(waiting: boolean): void {
    this.stop();
    this.ringing = true;
    this.waiting = waiting;
    void this.unlock();
  }

  private async tone(generation: number): Promise<void> {
    const context = this.context;
    if (!context || context.state !== "running") { this.blocked(true); return; }
    try {
      this.buffer ??= fetch(ringtoneUrl).then(response => {
        if (!response.ok) throw new Error("Could not load ringtone");
        return response.arrayBuffer();
      }).then(data => context.decodeAudioData(data)).catch(error => {
        this.buffer = undefined;
        throw error;
      });
      const buffer = await this.buffer;
      if (!this.ringing || generation !== this.generation || this.source) return;
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.loop = !this.waiting;
      source.connect(context.destination);
      source.onended = () => {
        source.disconnect();
        if (this.source === source) this.source = undefined;
      };
      this.source = source;
      source.start();
      this.blocked(false);
    } catch {
      if (this.ringing && generation === this.generation) this.blocked(true);
    }
  }

  stop(): void {
    this.ringing = false;
    this.generation++;
    this.source?.stop();
    this.source = undefined;
  }

  dispose(): void { this.stop(); void this.context?.close(); }
}

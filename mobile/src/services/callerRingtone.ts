/** Synthesizes a local alert; browser audio permission never controls invitations. */
export class CallerRingtone {
  private context?: AudioContext;
  private timer?: ReturnType<typeof setInterval>;
  private ringing = false;
  private readonly blocked: (value: boolean) => void;
  constructor(blocked: (value: boolean) => void) { this.blocked = blocked; }

  async unlock(): Promise<void> {
    try {
      this.context ??= new AudioContext();
      await this.context.resume();
      this.blocked(this.context.state !== "running");
      if (this.ringing) this.tone();
    } catch { this.blocked(true); }
  }

  play(waiting: boolean): void {
    this.stop(); this.ringing = true; this.tone();
    if (!waiting) this.timer = setInterval(() => this.tone(), 3000);
  }

  private tone(): void {
    if (!this.context || this.context.state !== "running") { this.blocked(true); return; }
    this.blocked(false);
    const start = this.context.currentTime;
    for (const offset of [0, 0.25]) {
      const oscillator = this.context.createOscillator();
      const volume = this.context.createGain();
      oscillator.frequency.value = 660;
      volume.gain.setValueAtTime(0, start + offset);
      volume.gain.linearRampToValueAtTime(0.12, start + offset + 0.02);
      volume.gain.linearRampToValueAtTime(0, start + offset + 0.2);
      oscillator.connect(volume); volume.connect(this.context.destination);
      oscillator.start(start + offset); oscillator.stop(start + offset + 0.22);
      oscillator.onended = () => { oscillator.disconnect(); volume.disconnect(); };
    }
  }

  stop(): void { this.ringing = false; clearInterval(this.timer); }
  dispose(): void { this.stop(); void this.context?.close(); }
}

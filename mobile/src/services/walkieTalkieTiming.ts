export function measureCaller(phase: string, start: number): void {
  const name = `Caller ${phase}`;
  try {
    performance.clearMeasures(name);
    performance.measure(name, { start, end: performance.now() });
  } catch {
    // Some browsers lack support for numeric User Timing options.
  }
}

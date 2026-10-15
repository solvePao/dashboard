export function generate128Hex(): string {
  const bytes = new Uint8Array(64);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}


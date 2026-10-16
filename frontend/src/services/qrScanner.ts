const CODE_PATTERN = /^[A-Za-z0-9]{6,128}$/;
const MAX_QR_LEN = 2048;

function sanitizeCandidate(raw: string | null): string {
  if (!raw) return "";
  const trimmed = raw.trim().slice(0, MAX_QR_LEN);
  return CODE_PATTERN.test(trimmed) ? trimmed : "";
}

export function extractSessionOrCode(decodedText: string): string {
  const input = decodedText.trim().slice(0, MAX_QR_LEN);
  if (!input) return "";
  try {
    const url = new URL(input);
    // Only accept http(s) URLs; never auto-authorize arbitrary schemes or
    // hosts. The captured param must itself match the code/session shape.
    if (url.protocol !== "http:" && url.protocol !== "https:") return "";
    const sessionParam = url.searchParams.get("session") || url.searchParams.get("code");
    const clean = sanitizeCandidate(sessionParam);
    if (clean) return clean;
    // A bare URL without a valid session/code param is not authorizable.
    // Fall through to raw-text check only when the whole input is not a URL
    // with an invalid param (avoid turning phishing URLs into codes).
    if (sessionParam !== null) return "";
  } catch {
    // Not a full URL, validate as raw text below.
  }
  return sanitizeCandidate(input);
}

type Html5QrcodeType = import("html5-qrcode").Html5Qrcode;

export class QrScannerManager {
  private scanner: Html5QrcodeType | null = null;
  private elementId: string;

  constructor(elementId: string) {
    this.elementId = elementId;
  }

  async start(
    onSuccess: (decodedText: string) => void,
    onError?: (errorMessage: string) => void
  ): Promise<void> {
    if (!this.scanner) {
      const { Html5Qrcode } = await import("html5-qrcode");
      this.scanner = new Html5Qrcode(this.elementId);
    }

    await this.scanner.start(
      { facingMode: "environment" },
      {
        fps: 10,
        qrbox: { width: 220, height: 220 },
      },
      (decodedText) => {
        onSuccess(extractSessionOrCode(decodedText));
      },
      (errorMessage) => {
        if (onError) {
          onError(errorMessage);
        }
      }
    );
  }

  async stop(): Promise<void> {
    if (this.scanner) {
      try {
        if (this.scanner.isScanning) {
          await this.scanner.stop();
        }
        await this.scanner.clear();
      } catch {
        // Ignore cleanup error
      }
      this.scanner = null;
    }
  }

  isScanning(): boolean {
    return this.scanner?.isScanning ?? false;
  }
}


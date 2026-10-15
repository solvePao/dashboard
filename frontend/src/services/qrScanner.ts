import { Html5Qrcode } from "html5-qrcode";

export function extractSessionOrCode(decodedText: string): string {
  try {
    const url = new URL(decodedText);
    const sessionParam = url.searchParams.get("session") || url.searchParams.get("code");
    if (sessionParam) return sessionParam;
  } catch {
    // Not a full URL, fallback to raw text
  }
  return decodedText.trim();
}

export class QrScannerManager {
  private scanner: Html5Qrcode | null = null;
  private elementId: string;

  constructor(elementId: string) {
    this.elementId = elementId;
  }

  async start(
    onSuccess: (decodedText: string) => void,
    onError?: (errorMessage: string) => void
  ): Promise<void> {
    if (!this.scanner) {
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


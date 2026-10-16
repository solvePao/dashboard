import { computed, ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import {
  createDeviceSessionApi,
  checkDeviceAuthApi,
  fetchMeApi,
  pairingSocketUrl,
  deviceSocketUrl,
} from "../api/deviceAuthApi";
import type { CheckAuthResponse } from "../types/auth";
import { generate128Hex } from "../services/crypto";
import {
  getStoredDeviceToken,
  setStoredDeviceToken,
  removeStoredDeviceToken,
} from "../services/deviceStorage";
import type { UserProfile, AuthState } from "../types/auth";

export type { UserProfile, AuthState };

const sessionId = ref<string>("");
const shortCode = ref<string>("");
const qrSvg = ref<string>("");
const expiresAt = ref<string>("");
const user = ref<UserProfile | null>(null);
const authState = ref<AuthState>("idle");
const statusMessage = ref<string>("");

let pairingSocket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
let reconnectAttempts = 0;

let deviceSocket: WebSocket | null = null;
let deviceReconnectTimer: ReturnType<typeof setTimeout> | undefined;
let deviceReconnectAttempts = 0;

/** Fallback display pairing code when the backend omits short_code. */
export function deriveShortCode(id: string): string {
  return id ? id.slice(0, 6).toUpperCase() : "";
}

/**
 * True only when the error proves the saved token was rejected (vs. the
 * backend being unreachable). Network failures must NOT clear the token.
 */
export function shouldClearStoredToken(err: unknown): boolean {
  return err instanceof ApiError && (err.status === 401 || err.status === 403);
}

export function useDeviceAuth() {
  const isAuthenticated = computed(() => authState.value === "authorized" && !!user.value);

  function closePairingSocket(): void {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = undefined;
    }
    reconnectAttempts = 0;
    if (pairingSocket) {
      pairingSocket.onclose = null;
      pairingSocket.close();
      pairingSocket = null;
    }
  }

  function closeDeviceSocket(): void {
    if (deviceReconnectTimer) {
      clearTimeout(deviceReconnectTimer);
      deviceReconnectTimer = undefined;
    }
    deviceReconnectAttempts = 0;
    if (deviceSocket) {
      deviceSocket.onclose = null;
      deviceSocket.close();
      deviceSocket = null;
    }
  }

  function connectDeviceSocket(token: string): void {
    if (!token || authState.value !== "authorized") return;
    if (deviceSocket && deviceSocket.readyState <= WebSocket.OPEN) return;

    const ws = new WebSocket(deviceSocketUrl(token));
    deviceSocket = ws;

    ws.onopen = () => {
      deviceReconnectAttempts = 0;
    };
    ws.onmessage = async (event) => {
      try {
        const body = JSON.parse(String(event.data)) as { status?: string; message?: string };
        if (body.status === "revoked") {
          logger.warn("auth", "Device revoked by web dashboard");
          closeDeviceSocket();
          removeStoredDeviceToken();
          user.value = null;
          await createSession();
          statusMessage.value =
            "Device access was revoked from the web dashboard. Scan QR code to reconnect.";
        }
      } catch {
        logger.warn("auth", "Ignored malformed device socket message");
      }
    };
    ws.onclose = () => {
      if (deviceSocket === ws) deviceSocket = null;
      if (authState.value === "authorized" && getStoredDeviceToken() === token) {
        const delay = Math.min(1000 * 2 ** deviceReconnectAttempts, 5000);
        deviceReconnectAttempts += 1;
        deviceReconnectTimer = setTimeout(() => connectDeviceSocket(token), delay);
      }
    };
    ws.onerror = () => ws.close();
  }

  function applyAuthorized(body: CheckAuthResponse): boolean {
    if (body.status !== "authorized" || !body.token || !body.user) return false;
    user.value = body.user;
    setStoredDeviceToken(body.token);
    authState.value = "authorized";
    statusMessage.value = "Successfully authorized!";
    logger.info("auth", "Device authorized successfully", { userId: body.user.id });
    closePairingSocket();
    connectDeviceSocket(body.token);
    return true;
  }

  function applyExpired(): void {
    authState.value = "expired";
    statusMessage.value = "Session has expired. Please regenerate a new QR code.";
    closePairingSocket();
  }

  function connectPairingSocket(): void {
    if (!sessionId.value || authState.value === "authorized") return;
    if (pairingSocket && pairingSocket.readyState <= WebSocket.OPEN) return;

    const ws = new WebSocket(pairingSocketUrl(sessionId.value));
    pairingSocket = ws;

    ws.onopen = () => {
      reconnectAttempts = 0;
    };
    ws.onmessage = (event) => {
      try {
        const body = JSON.parse(String(event.data)) as CheckAuthResponse;
        if (applyAuthorized(body)) return;
        if (body.status === "expired") applyExpired();
      } catch {
        logger.warn("auth", "Ignored malformed pairing message");
      }
    };
    ws.onclose = () => {
      if (pairingSocket === ws) pairingSocket = null;
      // Reconnect while the 2-minute pairing window is still open;
      // the object replays a finished result on reconnect.
      if (
        sessionId.value &&
        (authState.value === "pending" || authState.value === "checking")
      ) {
        const delay = Math.min(1000 * 2 ** reconnectAttempts, 5000);
        reconnectAttempts += 1;
        reconnectTimer = setTimeout(connectPairingSocket, delay);
      }
    };
    ws.onerror = () => ws.close();
  }

  async function restoreExistingToken(): Promise<boolean> {
    try {
      const stored = getStoredDeviceToken();
      if (!stored) return false;

      authState.value = "loading";
      statusMessage.value = "Verifying saved device token...";

      const body = await fetchMeApi(stored);
      user.value = body.user;
      authState.value = "authorized";
      statusMessage.value = "Device is connected and authenticated";
      logger.info("auth", "Restored session with device token", { userId: body.user.id });
      connectDeviceSocket(stored);
      return true;
    } catch (err) {
      if (shouldClearStoredToken(err)) {
        logger.warn("auth", "Saved token rejected; clearing");
        removeStoredDeviceToken();
      } else {
        const message = err instanceof Error ? err.message : "Network error occurred";
        logger.warn("auth", "Token verification unreachable; keeping saved token", message);
        authState.value = "error";
        statusMessage.value =
          "Cannot reach the backend. Your saved login was kept — tap Check for Auth to retry.";
      }
      return false;
    }
  }

  async function createSession(): Promise<void> {
    closePairingSocket();
    closeDeviceSocket();
    try {
      authState.value = "loading";
      statusMessage.value = "Generating session ID...";
      sessionId.value = "";
      shortCode.value = "";
      qrSvg.value = "";
      expiresAt.value = "";

      const newSessionId = generate128Hex();
      sessionId.value = newSessionId;

      const body = await createDeviceSessionApi(newSessionId);

      sessionId.value = body.session_id;
      shortCode.value = body.short_code || deriveShortCode(body.session_id);
      qrSvg.value = body.qr_svg;
      expiresAt.value = body.expires_at;
      authState.value = "pending";
      statusMessage.value = "Scan QR code or enter pairing code on the web dashboard.";
      connectPairingSocket();
      logger.info("auth", "Device pairing session created", {
        sessionId: body.session_id.slice(0, 8),
        shortCode: shortCode.value,
      });
    } catch (err) {
      authState.value = "error";
      const message = err instanceof Error ? err.message : "Failed to initialize pairing session";
      statusMessage.value = message;
      logger.error("auth", "Error creating session", message);
    }
  }

  async function init(): Promise<void> {
    const restored = await restoreExistingToken();
    if (restored) return;
    await createSession();
    if (getStoredDeviceToken()) {
      // Verification failed without rejecting the token (e.g. offline at
      // startup): the pairing session is ready, but the saved login is kept
      // and can be re-verified via Check for Auth.
      statusMessage.value =
        "Cannot reach the backend. Your saved login was kept — tap Check for Auth to retry.";
    }
  }

  async function checkAuth(): Promise<void> {
    // A saved-but-unverified token (e.g. offline at startup) is re-verified
    // first so the user has a retry path through the existing button.
    if (!isAuthenticated.value && getStoredDeviceToken()) {
      if (await restoreExistingToken()) return;
    }

    if (!sessionId.value) {
      await createSession();
      return;
    }

    if (authState.value === "authorized") return;

    authState.value = "checking";
    statusMessage.value = "Checking authorization status...";

    try {
      const body = await checkDeviceAuthApi(sessionId.value);

      if (applyAuthorized(body)) return;
      if (body.status === "expired") {
        applyExpired();
      } else {
        authState.value = "pending";
        statusMessage.value =
          "Device not authorized yet. Scan QR code or enter the 6-character code on the web dashboard — this page updates on its own once authorized.";
        connectPairingSocket();
      }
    } catch (err) {
      authState.value = "pending";
      const message = err instanceof Error ? err.message : "Failed to check authorization";
      statusMessage.value = message;
      logger.error("auth", "Check auth failed", message);
    }
  }

  async function disconnect(): Promise<void> {
    try {
      closeDeviceSocket();
      removeStoredDeviceToken();
      user.value = null;
      authState.value = "idle";
      statusMessage.value = "";
      logger.info("auth", "Device disconnected");
      await createSession();
    } catch (err) {
      logger.error("auth", "Disconnect error", err);
    }
  }

  return {
    sessionId,
    shortCode,
    qrSvg,
    expiresAt,
    user,
    authState,
    statusMessage,
    isAuthenticated,
    init,
    createSession,
    checkAuth,
    disconnect,
  };
}

import { computed, ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import {
  createDeviceSessionApi,
  checkDeviceAuthApi,
  disconnectDeviceApi,
  fetchMeApi,
  livePairingUrl,
  liveDeviceUrl,
} from "../api/deviceAuthApi";
import { liveSocket } from "../services/liveSocket";
import type { CheckAuthResponse } from "../types/auth";
import { generateSessionIdHex } from "../services/crypto";
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
const isDisconnecting = ref<boolean>(false);
const isConfirmingDisconnect = ref<boolean>(false);
const disconnectError = ref<string>("");

let pairingUnsubscribe: (() => void) | undefined;
let deviceUnsubscribe: (() => void) | undefined;

/** Fallback display pairing code when the backend omits short_code. */
export function deriveShortCode(id: string): string {
  return id ? id.slice(0, 6).toUpperCase() : "";
}

/// Observer hook for live device-socket pushes the auth flow itself
/// does not consume (e.g. `widgets_updated`). Widget sync subscribes
/// here instead of opening a second socket.
export type DeviceSocketEvent = {
  status?: string;
  message?: string;
  group_id?: string;
};

export type DeviceSocketListener = (event: DeviceSocketEvent) => void;

const deviceSocketListeners = new Set<DeviceSocketListener>();

export function subscribeDeviceSocket(listener: DeviceSocketListener): () => void {
  deviceSocketListeners.add(listener);
  return () => {
    deviceSocketListeners.delete(listener);
  };
}

function notifyDeviceSocketListeners(event: DeviceSocketEvent): void {
  for (const listener of deviceSocketListeners) {
    try {
      listener(event);
    } catch (err) {
      logger.warn("auth", "device socket listener failed", err);
    }
  }
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
    pairingUnsubscribe?.();
    pairingUnsubscribe = undefined;
  }

  function closeDeviceSocket(): void {
    deviceUnsubscribe?.();
    deviceUnsubscribe = undefined;
    liveSocket.disconnect();
  }

  function connectDeviceSocket(token: string): void {
    if (!token || authState.value !== "authorized") return;
    liveSocket.connect(liveDeviceUrl(token));
    if (deviceUnsubscribe) return;
    deviceUnsubscribe = liveSocket.subscribe("device", async (envelope) => {
      const body = envelope as DeviceSocketEvent;
      if (body.status === "revoked") {
        logger.warn("auth", "Device revoked by web dashboard");
        closeDeviceSocket();
        removeStoredDeviceToken();
        user.value = null;
        await createSession();
        statusMessage.value =
          "Device access was revoked from the web dashboard. Scan QR code to reconnect.";
      } else if (body.status === "widgets_updated" || body.status === "walkie_activate") {
        // Notify listeners (widgets refresh, walkie incoming call auto-activates)
        notifyDeviceSocketListeners(body);
      }
    });
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
    liveSocket.connect(livePairingUrl(sessionId.value));
    if (pairingUnsubscribe) return;
    pairingUnsubscribe = liveSocket.subscribe("pairing", (envelope) => {
      const body = envelope as unknown as CheckAuthResponse;
      if (applyAuthorized(body)) return;
      if (body.status === "expired") applyExpired();
    });
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
    isConfirmingDisconnect.value = false;
    disconnectError.value = "";
    closePairingSocket();
    closeDeviceSocket();
    try {
      authState.value = "loading";
      statusMessage.value = "Generating session ID...";
      sessionId.value = "";
      shortCode.value = "";
      qrSvg.value = "";
      expiresAt.value = "";

      const newSessionId = generateSessionIdHex();
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

  /**
   * Self-disconnect: the backend delete must confirm before anything
   * local is cleared, so "disconnected" here always means "deleted on
   * the backend". Already-gone responses (401/403/404) are the same
   * end state and count as confirmed; network/5xx failures keep the
   * session untouched and surface a retry message.
   */
  function requestDisconnect(): void {
    disconnectError.value = "";
    isConfirmingDisconnect.value = true;
  }

  function cancelDisconnect(): void {
    if (isDisconnecting.value) return;
    disconnectError.value = "";
    isConfirmingDisconnect.value = false;
  }

  async function disconnect(): Promise<void> {
    if (isDisconnecting.value) return;
    disconnectError.value = "";
    const token = getStoredDeviceToken();
    if (!token) {
      closeDeviceSocket();
      user.value = null;
      authState.value = "idle";
      statusMessage.value = "";
      isConfirmingDisconnect.value = false;
      await createSession();
      return;
    }

    isDisconnecting.value = true;
    try {
      await disconnectDeviceApi(token);
    } catch (err) {
      const alreadyGone =
        err instanceof ApiError && (err.status === 401 || err.status === 403 || err.status === 404);
      if (!alreadyGone) {
        disconnectError.value =
          "Could not disconnect from the backend. Check your connection and try again.";
        logger.error("auth", "Backend disconnect failed; staying connected", err);
        isDisconnecting.value = false;
        return;
      }
      logger.info("auth", "Device already deleted on backend; finishing disconnect");
    }

    closeDeviceSocket();
    removeStoredDeviceToken();
    user.value = null;
    authState.value = "idle";
    statusMessage.value = "";
    isDisconnecting.value = false;
    isConfirmingDisconnect.value = false;
    logger.info("auth", "Device disconnected");
    await createSession();
  }

  return {
    sessionId,
    shortCode,
    qrSvg,
    expiresAt,
    user,
    authState,
    statusMessage,
    isDisconnecting,
    isConfirmingDisconnect,
    disconnectError,
    isAuthenticated,
    subscribeDeviceSocket,
    init,
    createSession,
    checkAuth,
    requestDisconnect,
    cancelDisconnect,
    disconnect,
  };
}

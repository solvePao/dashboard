import { computed, ref } from "vue";
import { logger } from "../lib/logger";
import {
  createDeviceSessionApi,
  checkDeviceAuthApi,
  fetchMeApi,
} from "../api/deviceAuthApi";
import { generate128Hex } from "../services/crypto";
import {
  getStoredDeviceToken,
  setStoredDeviceToken,
  removeStoredDeviceToken,
} from "../services/deviceStorage";
import type { UserProfile, AuthState } from "../types/auth";

export type { UserProfile, AuthState };
export { generate128Hex };

const sessionId = ref<string>("");
const qrSvg = ref<string>("");
const expiresAt = ref<string>("");
const user = ref<UserProfile | null>(null);
const deviceToken = ref<string | null>(null);
const authState = ref<AuthState>("idle");
const statusMessage = ref<string>("");

export function useDeviceAuth() {
  const shortCode = computed(() => {
    return sessionId.value ? sessionId.value.slice(0, 6).toUpperCase() : "";
  });

  const isAuthenticated = computed(() => authState.value === "authorized" && !!user.value);

  async function restoreExistingToken(): Promise<boolean> {
    try {
      const stored = getStoredDeviceToken();
      if (!stored) return false;

      authState.value = "loading";
      statusMessage.value = "Verifying saved device token...";

      const body = await fetchMeApi(stored);
      user.value = body.user;
      deviceToken.value = stored;
      authState.value = "authorized";
      statusMessage.value = "Device is connected and authenticated";
      logger.info("auth", "Restored session with device token", { userId: body.user.id });
      return true;
    } catch {
      logger.warn("auth", "Saved token invalid or expired");
      removeStoredDeviceToken();
      deviceToken.value = null;
      return false;
    }
  }

  async function createSession(): Promise<void> {
    try {
      authState.value = "loading";
      statusMessage.value = "Generating session ID...";

      const newSessionId = generate128Hex();
      sessionId.value = newSessionId;

      const body = await createDeviceSessionApi(newSessionId);

      sessionId.value = body.session_id;
      qrSvg.value = body.qr_svg;
      expiresAt.value = body.expires_at;
      authState.value = "pending";
      statusMessage.value = "Scan QR code or enter pairing code on the web dashboard.";
      logger.info("auth", "Device pairing session created", {
        sessionId: body.session_id.slice(0, 8),
        shortCode: body.short_code,
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
    if (!restored) {
      await createSession();
    }
  }

  async function checkAuth(): Promise<void> {
    if (!sessionId.value) {
      await createSession();
      return;
    }

    if (authState.value === "authorized") return;

    authState.value = "checking";
    statusMessage.value = "Checking authorization status...";

    try {
      const body = await checkDeviceAuthApi(sessionId.value);

      if (body.status === "authorized" && body.token && body.user) {
        user.value = body.user;
        deviceToken.value = body.token;
        setStoredDeviceToken(body.token);
        authState.value = "authorized";
        statusMessage.value = "Successfully authorized!";
        logger.info("auth", "Device authorized successfully", { userId: body.user.id });
      } else if (body.status === "expired") {
        authState.value = "expired";
        statusMessage.value = "Session has expired. Please regenerate a new QR code.";
      } else {
        authState.value = "pending";
        statusMessage.value =
          "Device not authorized yet. Scan QR code or enter the 6-character code on the web dashboard, then click Check for Auth.";
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
      removeStoredDeviceToken();
      user.value = null;
      deviceToken.value = null;
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
    deviceToken,
    authState,
    statusMessage,
    isAuthenticated,
    init,
    createSession,
    checkAuth,
    disconnect,
  };
}

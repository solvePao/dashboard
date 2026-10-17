import { computed, ref } from "vue";
import { logger } from "../lib/logger";
import { ApiError } from "../api/client";
import { exchangeGoogleCredential, logoutApi } from "../api/authApi";
import {
  persistSession,
  restoreSession,
  clearSession,
} from "../services/sessionStorage";
import {
  renderGoogleButton,
  disableGoogleAutoSelect,
  googleClientId,
} from "../services/googleIdentity";
import { useDevices } from "./useDevices";
import type { SessionUser, StoredSession } from "../types/auth";
import type { DeviceItem, AuthorizeDeviceResult } from "../types/device";
import type { GoogleCredentialResponse } from "../types/gsi";

export type { SessionUser, StoredSession };
export type { DeviceItem };

const user = ref<SessionUser | null>(null);
/** Server-minted session JWT. The Google credential is never stored —
 *  it exists only as the argument of `exchangeCredential` below. */
const authToken = ref<string | null>(null);
const sessionExpiresAt = ref<number | null>(null);
const authError = ref<string | null>(null);
const authLoading = ref(false);

const {
  devices,
  devicesLoading,
  devicesError,
  fetchDevices: fetchDevicesAction,
  authorizeDevice: authorizeDeviceAction,
  revokeDevice: revokeDeviceAction,
  clearDevices,
} = useDevices();

function signOut(): void {
  // Expire the hub cookie server-side (best-effort); clearDevices
  // below closes the hub socket itself.
  void logoutApi().catch(() => undefined);
  disableGoogleAutoSelect();
  user.value = null;
  authToken.value = null;
  sessionExpiresAt.value = null;
  authError.value = null;
  clearDevices();
  clearSession();
  logger.info("auth", "signed out");
}

function requireValidToken(): string | null {
  const token = authToken.value;
  if (!token) return null;
  if (sessionExpiresAt.value !== null && sessionExpiresAt.value <= Date.now()) {
    signOut();
    authError.value = "Session expired. Please sign in again.";
    return null;
  }
  return token;
}

function handleUnauthorized(err: unknown): boolean {
  if (err instanceof ApiError && err.status === 401) {
    signOut();
    authError.value = "Session expired. Please sign in again.";
    return true;
  }
  return false;
}

async function exchangeCredential(credential: string): Promise<void> {
  authLoading.value = true;
  authError.value = null;
  try {
    const data = await exchangeGoogleCredential(credential);
    user.value = data.user;
    authToken.value = data.token;
    sessionExpiresAt.value = data.expires_at;
    persistSession({ user: data.user, expiresAt: data.expires_at, token: data.token });
    logger.info("auth", "login succeeded");
    await fetchDevicesAction(data.token).catch((err: unknown) => {
      handleUnauthorized(err);
    });
  } catch (err) {
    if (handleUnauthorized(err)) return;
    const message = err instanceof Error ? err.message : "Login failed.";
    logger.error("auth", "login failed", message);
    authError.value = message;
    user.value = null;
    authToken.value = null;
    sessionExpiresAt.value = null;
    clearSession();
  } finally {
    authLoading.value = false;
  }
}

function handleCredential(response: GoogleCredentialResponse): void {
  void exchangeCredential(response.credential);
}

async function initGoogleButton(
  element: HTMLElement,
  options?: { dark?: boolean }
): Promise<void> {
  try {
    await renderGoogleButton(element, handleCredential, options);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Failed to initialize Google login.";
    authError.value = message;
    throw err;
  }
}

function initSession(): void {
  const stored = restoreSession();
  if (stored) {
    user.value = stored.user;
    authToken.value = stored.token;
    sessionExpiresAt.value = stored.expiresAt;
    logger.info("auth", "restored session");
    void fetchDevicesAction(stored.token).catch((err: unknown) => {
      handleUnauthorized(err);
    });
  }
}

// Cross-tab sign-out sync: the server JWT lives in localStorage (short
// 60-minute expiry mitigates XSS persistence, but any XSS can still read it
// during its lifetime — httpOnly cookies would require a backend session
// refactor, out of scope here). When one tab signs out or its session is
// cleared/expired, other tabs sign out too instead of acting on a stale JWT.
if (typeof window !== "undefined") {
  window.addEventListener("storage", (event: StorageEvent) => {
    if (event.key === "dashboard.auth.session" && !event.newValue && authToken.value) {
      signOut();
    }
  });
}

initSession();

export function useAuth() {
  async function fetchDevices(): Promise<void> {
    const token = requireValidToken();
    if (!token) return;
    try {
      await fetchDevicesAction(token);
    } catch (err) {
      handleUnauthorized(err);
    }
  }

  async function authorizeDevice(
    codeOrSessionId: string,
    deviceName: string = "Mobile Device"
  ): Promise<AuthorizeDeviceResult> {
    const token = requireValidToken();
    if (!token) {
      return { success: false, message: authError.value || "Please sign in to authorize devices." };
    }
    try {
      return await authorizeDeviceAction(token, codeOrSessionId, deviceName);
    } catch (err) {
      if (handleUnauthorized(err)) {
        return { success: false, message: "Session expired. Please sign in again." };
      }
      throw err;
    }
  }

  async function revokeDevice(deviceToken: string): Promise<boolean> {
    const token = requireValidToken();
    if (!token) return false;
    try {
      return await revokeDeviceAction(token, deviceToken);
    } catch (err) {
      handleUnauthorized(err);
      return false;
    }
  }

  return {
    user,
    authToken,
    authError,
    authLoading,
    devices,
    devicesLoading,
    devicesError,
    isLoggedIn: computed(() => user.value !== null && authToken.value !== null),
    googleClientId,
    initGoogleButton,
    signOut,
    fetchDevices,
    authorizeDevice,
    revokeDevice,
  };
}

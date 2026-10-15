import { computed, ref } from "vue";
import { logger } from "../lib/logger";
import { exchangeGoogleCredential } from "../api/authApi";
import {
  persistSession,
  restoreSession,
  clearSession,
  decodeExpiry,
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
const currentCredential = ref<string | null>(null);
const authError = ref<string | null>(null);
const authLoading = ref(false);

const {
  devices,
  devicesLoading,
  fetchDevices: fetchDevicesAction,
  authorizeDevice: authorizeDeviceAction,
  revokeDevice: revokeDeviceAction,
  clearDevices,
} = useDevices();

async function exchangeCredential(credential: string): Promise<void> {
  authLoading.value = true;
  authError.value = null;
  try {
    const data = await exchangeGoogleCredential(credential);
    const expiresAt = decodeExpiry(credential) ?? Date.now() + 55 * 60 * 1000;
    user.value = data.user;
    currentCredential.value = credential;
    persistSession({ user: data.user, expiresAt, credential });
    logger.info("auth", "login succeeded", { userId: data.user.id });
    void fetchDevicesAction(credential);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Login failed.";
    logger.error("auth", "login failed", message);
    authError.value = message;
    user.value = null;
    currentCredential.value = null;
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

function signOut(): void {
  disableGoogleAutoSelect();
  user.value = null;
  currentCredential.value = null;
  authError.value = null;
  clearDevices();
  clearSession();
  logger.info("auth", "signed out");
}

function initSession(): void {
  const stored = restoreSession();
  if (stored) {
    user.value = stored.user;
    currentCredential.value = stored.credential || null;
    logger.info("auth", "restored session", { userId: stored.user.id });
    if (stored.credential) {
      void fetchDevicesAction(stored.credential);
    }
  }
}

initSession();

export function useAuth() {
  return {
    user,
    currentCredential,
    authError,
    authLoading,
    devices,
    devicesLoading,
    isLoggedIn: computed(() => user.value !== null),
    googleClientId,
    initGoogleButton,
    signOut,
    fetchDevices: () => fetchDevicesAction(currentCredential.value),
    authorizeDevice: (codeOrSessionId: string, deviceName: string = "Mobile Device"): Promise<AuthorizeDeviceResult> =>
      authorizeDeviceAction(currentCredential.value, codeOrSessionId, deviceName),
    revokeDevice: (token: string): Promise<boolean> =>
      revokeDeviceAction(currentCredential.value, token),
  };
}

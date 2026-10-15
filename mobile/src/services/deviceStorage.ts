const DEVICE_TOKEN_KEY = "dashboard.mobile.device_token";

export function getStoredDeviceToken(): string | null {
  try {
    return localStorage.getItem(DEVICE_TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setStoredDeviceToken(token: string): void {
  try {
    localStorage.setItem(DEVICE_TOKEN_KEY, token);
  } catch {
    // Ignore storage errors
  }
}

export function removeStoredDeviceToken(): void {
  try {
    localStorage.removeItem(DEVICE_TOKEN_KEY);
  } catch {
    // Ignore storage errors
  }
}


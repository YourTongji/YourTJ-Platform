import type { Account } from "@/lib/api/types";
import { clearMediaDeliveryUrlCache } from "@/lib/media-delivery-cache";

const ACCESS_TOKEN_KEY = "yourtj.accessToken";
const REFRESH_TOKEN_KEY = "yourtj.refreshToken";
const ACCOUNT_KEY = "yourtj.account";
export const AUTH_CLEARED_EVENT = "yourtj:auth-cleared";

export interface StoredAuth {
  accessToken: string;
  refreshToken: string;
  account: Account;
}

export function readAccessToken() {
  return localStorage.getItem(ACCESS_TOKEN_KEY);
}

export function readRefreshToken() {
  return localStorage.getItem(REFRESH_TOKEN_KEY);
}

export function readStoredAccount(): Account | null {
  const raw = localStorage.getItem(ACCOUNT_KEY);
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as Account;
  } catch {
    return null;
  }
}

export function writeAuth(auth: StoredAuth) {
  if (readStoredAccount()?.id !== auth.account.id) {
    clearMediaDeliveryUrlCache();
  }
  localStorage.setItem(ACCESS_TOKEN_KEY, auth.accessToken);
  localStorage.setItem(REFRESH_TOKEN_KEY, auth.refreshToken);
  localStorage.setItem(ACCOUNT_KEY, JSON.stringify(auth.account));
}

export function writeAccount(account: Account) {
  localStorage.setItem(ACCOUNT_KEY, JSON.stringify(account));
}

export function clearAuth() {
  clearMediaDeliveryUrlCache();
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
  localStorage.removeItem(ACCOUNT_KEY);
  window.dispatchEvent(new Event(AUTH_CLEARED_EVENT));
}

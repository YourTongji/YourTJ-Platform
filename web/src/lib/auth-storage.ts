import type { Account } from "@/lib/api/types";
import { clearMediaDeliveryUrlCache } from "@/lib/media-delivery-cache";
import {
  allowLocalForumDraftsForAccount,
  clearLocalForumDraftsForAccount,
} from "@/lib/local-forum-drafts";
import { randomUuid } from "@/lib/random";

const ACCESS_TOKEN_KEY = "yourtj.accessToken";
const REFRESH_TOKEN_KEY = "yourtj.refreshToken";
const ACCOUNT_KEY = "yourtj.account";
const CLIENT_INSTALLATION_ID_KEY = "yourtj.clientInstallationId";
const UUID_V4_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
export const AUTH_CLEARED_EVENT = "yourtj:auth-cleared";

let inMemoryClientInstallationId: string | null = null;

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

export function readOrCreateClientInstallationId() {
  try {
    const stored = localStorage.getItem(CLIENT_INSTALLATION_ID_KEY);
    if (stored && UUID_V4_PATTERN.test(stored)) {
      inMemoryClientInstallationId = stored;
      return stored;
    }
  } catch {
    if (inMemoryClientInstallationId) return inMemoryClientInstallationId;
  }

  if (!inMemoryClientInstallationId) {
    inMemoryClientInstallationId = randomUuid();
  }
  try {
    localStorage.setItem(CLIENT_INSTALLATION_ID_KEY, inMemoryClientInstallationId);
  } catch {
    // The in-memory identifier still bounds sessions for this page lifetime when storage is denied.
  }
  return inMemoryClientInstallationId;
}

export function writeAuth(auth: StoredAuth) {
  const previousAccountId = readStoredAccount()?.id;
  if (previousAccountId !== auth.account.id) {
    clearMediaDeliveryUrlCache();
    if (previousAccountId) void clearLocalForumDraftsForAccount(previousAccountId);
  }
  allowLocalForumDraftsForAccount(auth.account.id);
  localStorage.setItem(ACCESS_TOKEN_KEY, auth.accessToken);
  localStorage.setItem(REFRESH_TOKEN_KEY, auth.refreshToken);
  localStorage.setItem(ACCOUNT_KEY, JSON.stringify(auth.account));
}

export function writeAccount(account: Account) {
  localStorage.setItem(ACCOUNT_KEY, JSON.stringify(account));
}

export function clearAuth() {
  const previousAccountId = readStoredAccount()?.id;
  clearMediaDeliveryUrlCache();
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
  localStorage.removeItem(ACCOUNT_KEY);
  if (previousAccountId) void clearLocalForumDraftsForAccount(previousAccountId);
  window.dispatchEvent(new Event(AUTH_CLEARED_EVENT));
}

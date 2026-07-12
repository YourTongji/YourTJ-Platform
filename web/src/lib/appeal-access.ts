import { randomUuid } from "@/lib/random";

const APPEAL_ACCESS_KEY = "yourtj.appealAccess";

interface AppealAccessToken {
  accessToken: string;
  expiresAt: number;
}

export interface AppealAccessSession extends AppealAccessToken {
  cachePartition: string;
}

function isCachePartition(value: unknown): value is string {
  return typeof value === "string" && /^[a-zA-Z0-9-]{8,128}$/.test(value);
}

export function readAppealAccess() {
  try {
    const raw = sessionStorage.getItem(APPEAL_ACCESS_KEY);
    if (!raw) return null;
    const value = JSON.parse(raw) as Partial<AppealAccessSession>;
    if (
      typeof value.accessToken !== "string"
      || typeof value.expiresAt !== "number"
      || value.expiresAt <= Math.floor(Date.now() / 1_000)
    ) {
      sessionStorage.removeItem(APPEAL_ACCESS_KEY);
      return null;
    }
    const session: AppealAccessSession = {
      accessToken: value.accessToken,
      expiresAt: value.expiresAt,
      cachePartition: isCachePartition(value.cachePartition)
        ? value.cachePartition
        : randomUuid(),
    };
    if (session.cachePartition !== value.cachePartition) {
      sessionStorage.setItem(APPEAL_ACCESS_KEY, JSON.stringify(session));
    }
    return session;
  } catch {
    return null;
  }
}

export function writeAppealAccess(value: AppealAccessToken) {
  const session: AppealAccessSession = { ...value, cachePartition: randomUuid() };
  sessionStorage.setItem(APPEAL_ACCESS_KEY, JSON.stringify(session));
  return session;
}

export function clearAppealAccess() {
  sessionStorage.removeItem(APPEAL_ACCESS_KEY);
}

const APPEAL_ACCESS_KEY = "yourtj.appealAccess";

interface AppealAccess {
  accessToken: string;
  expiresAt: number;
}

export function readAppealAccessToken() {
  try {
    const raw = sessionStorage.getItem(APPEAL_ACCESS_KEY);
    if (!raw) return null;
    const value = JSON.parse(raw) as Partial<AppealAccess>;
    if (
      typeof value.accessToken !== "string"
      || typeof value.expiresAt !== "number"
      || value.expiresAt <= Math.floor(Date.now() / 1_000)
    ) {
      sessionStorage.removeItem(APPEAL_ACCESS_KEY);
      return null;
    }
    return value.accessToken;
  } catch {
    return null;
  }
}

export function writeAppealAccess(value: AppealAccess) {
  sessionStorage.setItem(APPEAL_ACCESS_KEY, JSON.stringify(value));
}

export function clearAppealAccess() {
  sessionStorage.removeItem(APPEAL_ACCESS_KEY);
}

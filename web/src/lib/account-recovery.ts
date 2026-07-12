import type { RecoveryCredential } from "@/lib/api/types";

const RECOVERY_SESSION_KEY = "yourtj.accountRecovery";

export function storeRecoveryCredential(credential: RecoveryCredential) {
  sessionStorage.setItem(RECOVERY_SESSION_KEY, JSON.stringify(credential));
}

export function clearRecoveryCredential() {
  sessionStorage.removeItem(RECOVERY_SESSION_KEY);
}

export function readRecoveryCredential(): RecoveryCredential | null {
  try {
    const raw = sessionStorage.getItem(RECOVERY_SESSION_KEY);
    if (!raw) return null;
    const credential = JSON.parse(raw) as RecoveryCredential;
    if (!credential.recoveryToken || credential.expiresAt <= Math.floor(Date.now() / 1_000)) {
      clearRecoveryCredential();
      return null;
    }
    return credential;
  } catch {
    clearRecoveryCredential();
    return null;
  }
}

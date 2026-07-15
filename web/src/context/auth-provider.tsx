import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { api } from "@/lib/api/endpoints";
import type { Account, AuthTokens, EmailCodePurpose } from "@/lib/api/types";
import {
  AUTH_CLEARED_EVENT,
  clearAuth,
  readAccessToken,
  readStoredAccount,
  writeAccount,
  writeAuth,
} from "@/lib/auth-storage";
import { discardAllForumOptimisticUpdates } from "@/lib/forum-cache";

interface AuthContextValue {
  account: Account | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  requestCode: (
    email: string,
    captchaToken: string,
    purpose: EmailCodePurpose,
  ) => Promise<void>;
  verifyEmail: (input: {
    email: string;
    code: string;
    purpose: EmailCodePurpose;
    handle?: string;
    password?: string;
  }) => Promise<void>;
  loginWithPassword: (input: { email: string; password: string }) => Promise<void>;
  acceptAuthTokens: (tokens: AuthTokens) => Promise<void>;
  refreshMe: () => Promise<void>;
  updateProfile: (input: { handle?: string; avatarUrl?: string }) => Promise<void>;
  clearSession: () => void;
  logout: () => Promise<void>;
  logoutAll: () => Promise<void>;
}

const AuthContext = React.createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();
  const [account, setAccount] = React.useState<Account | null>(() => readStoredAccount());
  const [isLoading, setIsLoading] = React.useState(Boolean(readAccessToken()));
  const activeAccountId = React.useRef(account?.id);

  const clearPrincipalQueries = React.useCallback(async () => {
    discardAllForumOptimisticUpdates(queryClient);
    await queryClient.cancelQueries();
    queryClient.clear();
  }, [queryClient]);

  const refreshMe = React.useCallback(async () => {
    if (!readAccessToken()) {
      if (activeAccountId.current || readStoredAccount()) {
        clearAuth();
        await clearPrincipalQueries();
      }
      activeAccountId.current = undefined;
      setAccount(null);
      setIsLoading(false);
      return;
    }
    try {
      const latest = await api.me();
      if (activeAccountId.current !== latest.id) {
        await clearPrincipalQueries();
      }
      writeAccount(latest);
      activeAccountId.current = latest.id;
      setAccount(latest);
    } catch {
      clearAuth();
      await clearPrincipalQueries();
      activeAccountId.current = undefined;
      setAccount(null);
    } finally {
      setIsLoading(false);
    }
  }, [clearPrincipalQueries]);

  React.useEffect(() => {
    void refreshMe();
  }, [refreshMe]);

  const acceptAuthTokens = React.useCallback(async (tokens: AuthTokens) => {
    await clearPrincipalQueries();
    writeAuth(tokens);
    activeAccountId.current = tokens.account.id;
    setAccount(tokens.account);
  }, [clearPrincipalQueries]);

  React.useEffect(() => {
    const clearAfterLocalCredentialLoss = () => {
      activeAccountId.current = undefined;
      setAccount(null);
      setIsLoading(false);
      void clearPrincipalQueries();
    };
    window.addEventListener(AUTH_CLEARED_EVENT, clearAfterLocalCredentialLoss);
    return () => window.removeEventListener(AUTH_CLEARED_EVENT, clearAfterLocalCredentialLoss);
  }, [clearPrincipalQueries]);

  React.useEffect(() => {
    const refreshAfterCrossTabAuthChange = (event: StorageEvent) => {
      if (event.storageArea === localStorage && event.key === "yourtj.account") {
        void refreshMe();
      }
    };
    window.addEventListener("storage", refreshAfterCrossTabAuthChange);
    return () => window.removeEventListener("storage", refreshAfterCrossTabAuthChange);
  }, [refreshMe]);

  const value = React.useMemo<AuthContextValue>(
    () => ({
      account,
      isAuthenticated: Boolean(account),
      isLoading,
      requestCode: async (email, captchaToken, purpose) => {
        await api.requestEmailCode(email, captchaToken, purpose);
        toast.success("验证码已发送");
      },
      verifyEmail: async (input) => {
        const tokens = await api.verifyEmail(input);
        await acceptAuthTokens(tokens);
        toast.success("已登录 YourTJ");
      },
      loginWithPassword: async (input) => {
        const tokens = await api.passwordLogin(input);
        await acceptAuthTokens(tokens);
        toast.success("已登录 YourTJ");
      },
      acceptAuthTokens,
      refreshMe,
      updateProfile: async (input) => {
        const updated = await api.updateMe(input);
        writeAccount(updated);
        activeAccountId.current = updated.id;
        setAccount(updated);
        toast.success("资料已更新");
      },
      clearSession: () => {
        clearAuth();
        setAccount(null);
      },
      logout: async () => {
        try {
          await api.logout();
        } finally {
          clearAuth();
          await clearPrincipalQueries();
          activeAccountId.current = undefined;
          setAccount(null);
          toast.success("已退出登录");
        }
      },
      logoutAll: async () => {
        try {
          await api.logoutAll();
        } finally {
          clearAuth();
          await clearPrincipalQueries();
          activeAccountId.current = undefined;
          setAccount(null);
          toast.success("所有设备均已退出登录");
        }
      },
    }),
    [acceptAuthTokens, account, clearPrincipalQueries, isLoading, refreshMe],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const value = React.useContext(AuthContext);
  if (!value) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return value;
}

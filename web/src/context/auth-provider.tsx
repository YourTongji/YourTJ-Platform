import * as React from "react";
import { toast } from "sonner";

import { api } from "@/lib/api/endpoints";
import type { Account, EmailCodePurpose } from "@/lib/api/types";
import {
  clearAuth,
  readAccessToken,
  readStoredAccount,
  writeAccount,
  writeAuth,
} from "@/lib/auth-storage";

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
  refreshMe: () => Promise<void>;
  updateProfile: (input: { handle?: string; avatarUrl?: string }) => Promise<void>;
  logout: () => Promise<void>;
  logoutAll: () => Promise<void>;
}

const AuthContext = React.createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [account, setAccount] = React.useState<Account | null>(() => readStoredAccount());
  const [isLoading, setIsLoading] = React.useState(Boolean(readAccessToken()));

  const refreshMe = React.useCallback(async () => {
    if (!readAccessToken()) {
      setIsLoading(false);
      return;
    }
    try {
      const latest = await api.me();
      writeAccount(latest);
      setAccount(latest);
    } catch {
      clearAuth();
      setAccount(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void refreshMe();
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
        writeAuth(tokens);
        setAccount(tokens.account);
        toast.success("已登录 YourTJ");
      },
      loginWithPassword: async (input) => {
        const tokens = await api.passwordLogin(input);
        writeAuth(tokens);
        setAccount(tokens.account);
        toast.success("已登录 YourTJ");
      },
      refreshMe,
      updateProfile: async (input) => {
        const updated = await api.updateMe(input);
        writeAccount(updated);
        setAccount(updated);
        toast.success("资料已更新");
      },
      logout: async () => {
        try {
          await api.logout();
        } finally {
          clearAuth();
          setAccount(null);
          toast.success("已退出登录");
        }
      },
      logoutAll: async () => {
        try {
          await api.logoutAll();
        } finally {
          clearAuth();
          setAccount(null);
          toast.success("所有设备均已退出登录");
        }
      },
    }),
    [account, isLoading, refreshMe],
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

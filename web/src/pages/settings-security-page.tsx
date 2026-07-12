import { useMutation, useQuery } from "@tanstack/react-query";
import { Eye, EyeOff, KeyRound, Lock, Shield } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { RecentAuthStatus } from "@/lib/api/types";

function relativeTime(epochMs: number | null): string {
  if (epochMs === null) return "—";
  const diff = Date.now() - epochMs;
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  return `${days} 天前`;
}

function formatDateTime(epochMs: number | null): string {
  if (epochMs === null) return "—";
  return new Date(epochMs).toLocaleString("zh-CN", {
    hour12: false,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function PasswordInput({
  id,
  value,
  onChange,
  autoComplete,
  placeholder,
  disabled,
}: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete: string;
  placeholder?: string;
  disabled?: boolean;
}) {
  const [isVisible, setIsVisible] = React.useState(false);
  return (
    <div className="relative">
      <Input
        id={id}
        type={isVisible ? "text" : "password"}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        autoComplete={autoComplete}
        placeholder={placeholder}
        className="pr-11"
        minLength={8}
        maxLength={128}
        disabled={disabled}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="absolute right-1 top-1/2 size-8 -translate-y-1/2 text-muted-foreground"
        onClick={() => setIsVisible((current) => !current)}
        aria-label={isVisible ? "隐藏密码" : "显示密码"}
        disabled={disabled}
      >
        {isVisible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
      </Button>
    </div>
  );
}

function PasswordStatusBadge({ hasPassword }: { hasPassword: boolean }) {
  return (
    <div className="flex items-center gap-2">
      <Lock className="size-4 text-muted-foreground" aria-hidden="true" />
      <span className="text-sm text-muted-foreground">密码状态：</span>
      <Badge variant={hasPassword ? "default" : "secondary"}>
        {hasPassword ? "已设置" : "未设置"}
      </Badge>
    </div>
  );
}

function RecentAuthCard({ status }: { status: RecentAuthStatus | undefined }) {
  const { account } = useAuth();
  const [requestingCode, setRequestingCode] = React.useState(false);
  const [code, setCode] = React.useState("");

  const requestCode = useMutation({
    mutationFn: () => api.requestRecentAuthCode(),
    onSuccess: () => {
      setRequestingCode(true);
      toast.success("验证码已发送到你的邮箱");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发送失败"),
  });

  const verifyCode = useMutation({
    mutationFn: () => api.verifyRecentAuth({ method: "email_code", code: code.trim() }),
    onSuccess: () => {
      setCode("");
      setRequestingCode(false);
      toast.success("身份验证成功");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "验证失败"),
  });

  const isFresh = status?.isFresh ?? false;
  const methodLabel = status?.method === "password" ? "密码" : status?.method === "email_code" ? "验证码" : null;
  const hasPasswordInMethods = status?.availableMethods.includes("password") ?? false;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="size-5 text-primary" aria-hidden="true" />
          最近认证状态
        </CardTitle>
        <CardDescription>
          执行敏感操作（如修改密码）需要近期认证的会话来确认身份。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-muted-foreground">账号邮箱</span>
            <p className="font-medium">{account?.handle ?? "—"}@tongji.edu.cn</p>
          </div>
          <div>
            <span className="text-muted-foreground">认证状态</span>
            <p className="font-medium">
              {isFresh ? (
                <Badge variant="default" className="bg-green-600 hover:bg-green-600">已验证</Badge>
              ) : (
                <Badge variant="secondary">未验证</Badge>
              )}
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">认证方式</span>
            <p className="font-medium">{methodLabel ?? "—"}</p>
          </div>
          <div>
            <span className="text-muted-foreground">认证时间</span>
            <p className="font-medium">{formatDateTime(status?.authenticatedAt ?? null)}</p>
          </div>
          <div>
            <span className="text-muted-foreground">有效期至</span>
            <p className="font-medium">{relativeTime(status?.expiresAt ?? null)}</p>
          </div>
          <div>
            <span className="text-muted-foreground">会话绑定</span>
            <p className="font-medium">{status?.sessionBound ? "是" : "否"}</p>
          </div>
        </div>

        {!isFresh ? (
          <form
            className="space-y-3 rounded-lg border bg-muted/30 p-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (code.trim()) {
                verifyCode.mutate();
              } else {
                requestCode.mutate();
              }
            }}
          >
            <p className="text-sm font-medium">验证身份以执行敏感操作</p>
            {requestingCode ? (
              <div className="flex flex-col gap-2 sm:flex-row">
                <Input
                  value={code}
                  onChange={(event) => setCode(event.target.value)}
                  placeholder="输入验证码"
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  required
                />
                <Button
                  type="submit"
                  disabled={!code.trim() || verifyCode.isPending}
                >
                  {verifyCode.isPending ? "验证中" : "验证"}
                </Button>
              </div>
            ) : (
              <Button
                type="submit"
                variant="secondary"
                disabled={requestCode.isPending}
              >
                {requestCode.isPending ? "发送中" : "发送验证码到邮箱"}
              </Button>
            )}
            <p className="text-xs text-muted-foreground">
              验证码将发送到你的绑定邮箱。如未收到，请检查垃圾邮件。
            </p>
          </form>
        ) : (
          <div className="rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-800">
            当前会话已通过近期认证，可以执行敏感操作。
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          <Badge variant={hasPasswordInMethods ? "default" : "secondary"}>
            密码认证{hasPasswordInMethods ? " 可用" : " 不可用"}
          </Badge>
          <Badge variant="outline">
            邮箱验证码认证可用
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}

export function SettingsSecurityPage() {
  const { data: authStatus, refetch: refetchAuthStatus } = useQuery({
    queryKey: ["recentAuthStatus"],
    queryFn: () => api.recentAuthStatus(),
  });

  const hasPassword = authStatus?.availableMethods.includes("password") ?? false;

  // Change password form (for users with password)
  const [currentPassword, setCurrentPassword] = React.useState("");
  const [newPassword, setNewPassword] = React.useState("");
  const [confirmNewPassword, setConfirmNewPassword] = React.useState("");

  // Set password form (for users without password)
  const [setupPassword, setSetupPassword] = React.useState("");
  const [setupConfirmPassword, setSetupConfirmPassword] = React.useState("");

  const passwordsMatch = newPassword.length >= 8 && newPassword === confirmNewPassword;
  const setupPasswordsMatch = setupPassword.length >= 8 && setupPassword === setupConfirmPassword;

  const changePassword = useMutation({
    mutationFn: () => api.passwordChange({ currentPassword, newPassword }),
    onSuccess: () => {
      toast.success("密码已更新");
      setCurrentPassword("");
      setNewPassword("");
      setConfirmNewPassword("");
      void refetchAuthStatus();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "修改密码失败"),
  });

  const setPassword = useMutation({
    mutationFn: () => api.passwordChange({ currentPassword: "", newPassword: setupPassword }),
    onSuccess: () => {
      toast.success("密码已设置");
      setSetupPassword("");
      setSetupConfirmPassword("");
      void refetchAuthStatus();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "设置密码失败"),
  });

  return (
    <div className="mx-auto max-w-lg space-y-6">
      <PageHeader
        eyebrow="Settings"
        title="安全设置"
        description="密码管理、最近认证和账号安全选项。"
      />

      {/* Password management card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="size-5 text-primary" aria-hidden="true" />
            密码管理
          </CardTitle>
          <CardDescription>
            {hasPassword
              ? "定期更新密码有助于保护账号安全。"
              : "设置一个密码，之后即可直接使用密码登录。"}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <PasswordStatusBadge hasPassword={hasPassword} />

          <Separator />

          {hasPassword ? (
            /* Change password form */
            <form
              className="space-y-4"
              onSubmit={(event) => {
                event.preventDefault();
                changePassword.mutate();
              }}
            >
              <div className="space-y-2">
                <Label htmlFor="current-password">当前密码</Label>
                <PasswordInput
                  id="current-password"
                  value={currentPassword}
                  onChange={setCurrentPassword}
                  autoComplete="current-password"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="new-password">新密码</Label>
                <PasswordInput
                  id="new-password"
                  value={newPassword}
                  onChange={setNewPassword}
                  autoComplete="new-password"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="confirm-new-password">确认新密码</Label>
                <PasswordInput
                  id="confirm-new-password"
                  value={confirmNewPassword}
                  onChange={setConfirmNewPassword}
                  autoComplete="new-password"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                密码需为 8–128 个字符，并避免使用邮箱、姓名或常见组合。
              </p>
              <Button
                type="submit"
                className="w-full"
                disabled={
                  !currentPassword
                  || !passwordsMatch
                  || changePassword.isPending
                }
              >
                <Lock className="size-4" />
                {changePassword.isPending ? "正在更新" : "更新密码"}
              </Button>
            </form>
          ) : (
            /* Set password form (for users without password) */
            <form
              className="space-y-4"
              onSubmit={(event) => {
                event.preventDefault();
                if (!authStatus?.isFresh) {
                  toast.error("请先通过上方的身份验证后再设置密码");
                  return;
                }
                setPassword.mutate();
              }}
            >
              <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
                你尚未设置密码。设置密码后，你可以直接使用密码登录，无需每次都收取验证码。
              </div>
              <div className="space-y-2">
                <Label htmlFor="set-new-password">新密码</Label>
                <PasswordInput
                  id="set-new-password"
                  value={setupPassword}
                  onChange={setSetupPassword}
                  autoComplete="new-password"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="set-confirm-password">确认新密码</Label>
                <PasswordInput
                  id="set-confirm-password"
                  value={setupConfirmPassword}
                  onChange={setSetupConfirmPassword}
                  autoComplete="new-password"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                密码需为 8–128 个字符，并避免使用邮箱、姓名或常见组合。
              </p>
              <Button
                type="submit"
                className="w-full"
                disabled={
                  !setupPasswordsMatch
                  || setPassword.isPending
                  || !authStatus?.isFresh
                }
              >
                <Lock className="size-4" />
                {setPassword.isPending ? "正在设置" : "设置密码"}
              </Button>
            </form>
          )}
        </CardContent>
      </Card>

      {/* Recent auth status card */}
      <RecentAuthCard status={authStatus} />
    </div>
  );
}

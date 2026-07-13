import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Laptop, LogOut, ShieldAlert, Smartphone } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { RecentAuthDialog } from "@/components/auth/recent-auth-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";
import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";


function summarizeDeviceLabel(label?: string | null) {
  if (!label?.trim()) return "未命名会话";
  const raw = label.trim();
  const browser =
    /Edg\//.test(raw) ? "Edge"
    : /Chrome\//.test(raw) && !/Edg\//.test(raw) ? "Chrome"
    : /Firefox\//.test(raw) ? "Firefox"
    : /Safari\//.test(raw) && !/Chrome\//.test(raw) ? "Safari"
    : null;
  const system =
    /Android/i.test(raw) ? "Android"
    : /iPhone|iPad|iPod/i.test(raw) ? "iOS"
    : /Windows/i.test(raw) ? "Windows"
    : /Mac OS X|Macintosh/i.test(raw) ? "macOS"
    : /Linux/i.test(raw) ? "Linux"
    : null;
  if (browser && system) return `${browser} · ${system}`;
  if (browser) return browser;
  if (system) return system;
  return raw.length > 48 ? `${raw.slice(0, 45)}…` : raw;
}

function PasswordField({
  id,
  value,
  onChange,
  autoComplete,
}: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete: string;
}) {
  return (
    <Input
      id={id}
      type="password"
      value={value}
      onChange={(event) => onChange(event.target.value)}
      autoComplete={autoComplete}
      minLength={8}
      maxLength={128}
    />
  );
}

export function SecuritySettings() {
  const queryClient = useQueryClient();
  const { account, acceptAuthTokens, logoutAll } = useAuth();
  const [currentPassword, setCurrentPassword] = React.useState("");
  const [newPassword, setNewPassword] = React.useState("");
  const [confirmPassword, setConfirmPassword] = React.useState("");
  const [confirmAllDevices, setConfirmAllDevices] = React.useState(false);
  const [recentAuthOpen, setRecentAuthOpen] = React.useState(false);
  const hasPassword = account?.hasPassword ?? true;

  const sessions = useInfiniteQuery({
    queryKey: ["device-sessions"],
    queryFn: ({ pageParam }) => api.sessions(pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor ?? undefined,
  });
  const passwordMutation = useMutation({
    mutationFn: (mode: "set" | "change") => mode === "set"
      ? api.passwordSet({ newPassword })
      : api.passwordChange({ currentPassword, newPassword }),
    onSuccess: async (tokens, mode) => {
      await acceptAuthTokens(tokens);
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      toast.success(mode === "set" ? "密码已设置，旧会话已替换" : "密码已更新，其他登录会话已撤销");
      await queryClient.invalidateQueries({ queryKey: ["device-sessions"] });
    },
    onError: (error, mode) => {
      if (mode === "set" && error instanceof ApiError && error.status === 428) {
        setRecentAuthOpen(true);
        return;
      }
      toast.error(error instanceof Error ? error.message : "密码修改失败");
    },
  });
  const revokeSession = useMutation({
    mutationFn: (id: string) => api.revokeSession(id),
    onSuccess: async () => {
      toast.success("登录会话已撤销");
      await queryClient.invalidateQueries({ queryKey: ["device-sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "撤销失败"),
  });
  const revokeOthers = useMutation({
    mutationFn: () => api.revokeOtherSessions(),
    onSuccess: async () => {
      toast.success("其他会话均已退出登录");
      await queryClient.invalidateQueries({ queryKey: ["device-sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "撤销失败"),
  });

  const deviceSessions = sessions.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const passwordsMatch = newPassword.length >= 8 && newPassword === confirmPassword;

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="size-5 text-primary" aria-hidden="true" />
            密码安全
          </CardTitle>
          <CardDescription>
            {hasPassword
              ? "修改密码会替换当前会话，并撤销其他会话的访问权限。"
              : "首次设置密码需要重新验证校园邮箱，完成后会替换旧会话。"}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
              passwordMutation.mutate(hasPassword ? "change" : "set");
            }}
          >
            {hasPassword ? (
              <div className="space-y-2">
                <Label htmlFor="current-password">当前密码</Label>
                <PasswordField
                  id="current-password"
                  value={currentPassword}
                  onChange={setCurrentPassword}
                  autoComplete="current-password"
                />
              </div>
            ) : null}
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="settings-new-password">新密码</Label>
                <PasswordField
                  id="settings-new-password"
                  value={newPassword}
                  onChange={setNewPassword}
                  autoComplete="new-password"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="settings-confirm-password">确认新密码</Label>
                <PasswordField
                  id="settings-confirm-password"
                  value={confirmPassword}
                  onChange={setConfirmPassword}
                  autoComplete="new-password"
                />
              </div>
            </div>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="text-xs text-muted-foreground">
                {hasPassword
                  ? <>忘记当前密码？可在<Link className="mx-1 text-primary underline-offset-4 hover:underline" to="/login">登录页</Link>通过邮箱验证码安全重置。</>
                  : "设置后可以直接使用校园邮箱和密码登录。"}
              </p>
              <Button
                type="submit"
                disabled={(hasPassword && !currentPassword) || !passwordsMatch || passwordMutation.isPending}
              >
                {passwordMutation.isPending ? "正在更新" : hasPassword ? "修改密码" : "设置密码"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Laptop className="size-5 text-primary" aria-hidden="true" />
            登录会话
          </CardTitle>
          <CardDescription>按登录会话展示。每次完整登录都会创建新会话；若同一账号在多台设备保持登录，列表中会同时出现多条。只展示可读标签和有限时间信息，不保存可见的精确 IP 历史。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {sessions.isLoading ? (
            <p className="text-sm text-muted-foreground" role="status">正在加载登录会话</p>
          ) : sessions.isError ? (
            <div className="flex flex-wrap items-center justify-between gap-3" role="alert">
              <p className="text-sm text-destructive">会话列表加载失败。</p>
              <Button type="button" variant="outline" size="sm" onClick={() => void sessions.refetch()}>重试</Button>
            </div>
          ) : deviceSessions.length === 0 ? (
            <p className="text-sm text-muted-foreground">当前没有可管理的登录会话。</p>
          ) : (
            <ul className="divide-y rounded-lg border" aria-label="登录会话列表">
              {deviceSessions.map((session) => (
                <li key={session.id} className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="flex min-w-0 gap-3">
                    <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary" aria-hidden="true">
                      {session.deviceLabel?.toLowerCase().includes("mobile")
                        ? <Smartphone className="size-4" />
                        : <Laptop className="size-4" />}
                    </span>
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">
                        {summarizeDeviceLabel(session.deviceLabel)}
                        {session.isCurrent ? <span className="ml-2 text-xs text-primary">当前会话</span> : null}
                      </p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        最近使用 {formatUnixTime(session.lastUsedAt)} · 到期 {formatUnixTime(session.expiresAt)}
                      </p>
                    </div>
                  </div>
                  {!session.isCurrent ? (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => revokeSession.mutate(session.id)}
                      disabled={revokeSession.isPending}
                    >
                      撤销
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
          {sessions.hasNextPage ? (
            <Button
              type="button"
              variant="ghost"
              className="w-full"
              onClick={() => void sessions.fetchNextPage()}
              disabled={sessions.isFetchingNextPage}
            >
              {sessions.isFetchingNextPage ? "正在加载" : "加载更多会话"}
            </Button>
          ) : null}
          <div className="flex flex-col gap-2 border-t pt-4 sm:flex-row sm:justify-between">
            <Button
              type="button"
              variant="outline"
              onClick={() => revokeOthers.mutate()}
              disabled={revokeOthers.isPending}
            >
              <LogOut className="size-4" />
              撤销其他会话
            </Button>
            {!confirmAllDevices ? (
              <Button type="button" variant="destructive" onClick={() => setConfirmAllDevices(true)}>
                <ShieldAlert className="size-4" />
                退出所有会话
              </Button>
            ) : (
              <div className="flex flex-wrap items-center justify-end gap-2">
                <span className="text-sm text-destructive">包括当前会话，确定吗？</span>
                <Button type="button" variant="ghost" onClick={() => setConfirmAllDevices(false)}>取消</Button>
                <Button type="button" variant="destructive" onClick={() => void logoutAll()}>确定退出</Button>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      <RecentAuthDialog
        open={recentAuthOpen}
        onOpenChange={setRecentAuthOpen}
        description="首次设置密码会建立新的长期登录凭据，需要当前会话重新验证校园邮箱。"
        onVerified={() => passwordMutation.mutate("set")}
      />
    </>
  );
}

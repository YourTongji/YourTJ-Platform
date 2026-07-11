import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Laptop, LogOut, ShieldAlert, Smartphone } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

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
  const { logoutAll } = useAuth();
  const [currentPassword, setCurrentPassword] = React.useState("");
  const [newPassword, setNewPassword] = React.useState("");
  const [confirmPassword, setConfirmPassword] = React.useState("");
  const [confirmAllDevices, setConfirmAllDevices] = React.useState(false);

  const sessions = useInfiniteQuery({
    queryKey: ["device-sessions"],
    queryFn: ({ pageParam }) => api.sessions(pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.nextCursor ?? undefined,
  });
  const changePassword = useMutation({
    mutationFn: () => api.passwordChange({ currentPassword, newPassword }),
    onSuccess: async () => {
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      toast.success("密码已更新，其他设备会话已撤销");
      await queryClient.invalidateQueries({ queryKey: ["device-sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "密码修改失败"),
  });
  const revokeSession = useMutation({
    mutationFn: (id: string) => api.revokeSession(id),
    onSuccess: async () => {
      toast.success("设备会话已撤销");
      await queryClient.invalidateQueries({ queryKey: ["device-sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "撤销失败"),
  });
  const revokeOthers = useMutation({
    mutationFn: () => api.revokeOtherSessions(),
    onSuccess: async () => {
      toast.success("其他设备均已退出登录");
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
          <CardDescription>修改密码会保留当前设备，并撤销其他设备的访问权限。</CardDescription>
        </CardHeader>
        <CardContent>
          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
              changePassword.mutate();
            }}
          >
            <div className="space-y-2">
              <Label htmlFor="current-password">当前密码</Label>
              <PasswordField
                id="current-password"
                value={currentPassword}
                onChange={setCurrentPassword}
                autoComplete="current-password"
              />
            </div>
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
                还没有密码？退出后可在<Link className="mx-1 text-primary underline-offset-4 hover:underline" to="/login">验证码登录</Link>中选择“首次设置密码”。
              </p>
              <Button
                type="submit"
                disabled={!currentPassword || !passwordsMatch || changePassword.isPending}
              >
                {changePassword.isPending ? "正在更新" : "修改密码"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Laptop className="size-5 text-primary" aria-hidden="true" />
            登录设备
          </CardTitle>
          <CardDescription>只展示设备标签和有限时间信息，不保存可见的精确 IP 历史。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {sessions.isLoading ? (
            <p className="text-sm text-muted-foreground" role="status">正在加载设备会话</p>
          ) : sessions.isError ? (
            <div className="flex flex-wrap items-center justify-between gap-3" role="alert">
              <p className="text-sm text-destructive">设备列表加载失败。</p>
              <Button type="button" variant="outline" size="sm" onClick={() => void sessions.refetch()}>重试</Button>
            </div>
          ) : deviceSessions.length === 0 ? (
            <p className="text-sm text-muted-foreground">当前没有可管理的设备会话。</p>
          ) : (
            <ul className="divide-y rounded-lg border" aria-label="登录设备列表">
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
                        {session.deviceLabel || "未命名设备"}
                        {session.isCurrent ? <span className="ml-2 text-xs text-primary">当前设备</span> : null}
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
              {sessions.isFetchingNextPage ? "正在加载" : "加载更多设备"}
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
              撤销其他设备
            </Button>
            {!confirmAllDevices ? (
              <Button type="button" variant="destructive" onClick={() => setConfirmAllDevices(true)}>
                <ShieldAlert className="size-4" />
                退出所有设备
              </Button>
            ) : (
              <div className="flex flex-wrap items-center justify-end gap-2">
                <span className="text-sm text-destructive">包括当前设备，确定吗？</span>
                <Button type="button" variant="ghost" onClick={() => setConfirmAllDevices(false)}>取消</Button>
                <Button type="button" variant="destructive" onClick={() => void logoutAll()}>确定退出</Button>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </>
  );
}

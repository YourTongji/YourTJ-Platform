import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, MailCheck, ShieldCheck } from "lucide-react";
import * as React from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { RecentAuthMethod } from "@/lib/api/types";

export function RecentAuthDialog({
  open,
  onOpenChange,
  onVerified,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onVerified: () => void;
}) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { logout } = useAuth();
  const [method, setMethod] = React.useState<RecentAuthMethod>("password");
  const [password, setPassword] = React.useState("");
  const [code, setCode] = React.useState("");
  const [sentAt, setSentAt] = React.useState<number | null>(null);
  const [clock, setClock] = React.useState(() => Date.now());

  const status = useQuery({
    queryKey: ["recent-auth"],
    queryFn: () => api.recentAuthStatus(),
    enabled: open,
    staleTime: 0,
  });

  React.useEffect(() => {
    if (!open) {
      setPassword("");
      setCode("");
      setSentAt(null);
      return;
    }
    const methods = status.data?.availableMethods ?? [];
    if (methods.length > 0 && !methods.includes(method)) {
      setMethod(methods[0]);
    }
  }, [method, open, status.data?.availableMethods]);

  React.useEffect(() => {
    if (!open || sentAt === null) return undefined;
    const timer = window.setInterval(() => setClock(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [open, sentAt]);

  const sendCode = useMutation({
    mutationFn: () => api.requestRecentAuthCode(),
    onSuccess: () => {
      setSentAt(Date.now());
      setClock(Date.now());
      toast.success("安全验证码已发送");
    },
  });
  const verify = useMutation({
    mutationFn: () =>
      api.verifyRecentAuth(
        method === "password"
          ? { method, password }
          : { method, code },
      ),
    onSuccess: async (nextStatus) => {
      queryClient.setQueryData(["recent-auth"], nextStatus);
      setPassword("");
      setCode("");
      toast.success("当前设备已完成安全验证");
      onOpenChange(false);
      onVerified();
      await queryClient.invalidateQueries({ queryKey: ["recent-auth"] });
    },
  });

  const availableMethods = status.data?.availableMethods ?? [];
  const resendSeconds = sentAt === null
    ? 0
    : Math.max(0, 60 - Math.floor((clock - sentAt) / 1_000));
  const error = verify.error ?? sendCode.error;

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => {
      if (!verify.isPending && !sendCode.isPending) onOpenChange(nextOpen);
    }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-primary" aria-hidden="true" />
            确认是你本人
          </DialogTitle>
          <DialogDescription>
            角色、封禁和强制注销等高风险操作需要当前设备在最近 10 分钟内重新验证。
          </DialogDescription>
        </DialogHeader>

        {status.isLoading || status.isFetching ? (
          <p className="text-sm text-muted-foreground" role="status">正在检查当前设备</p>
        ) : status.isError ? (
          <div className="space-y-3" role="alert">
            <p className="text-sm text-destructive">无法读取安全验证状态，请检查网络后重试。</p>
            <Button type="button" variant="outline" onClick={() => void status.refetch()}>重试</Button>
          </div>
        ) : !status.data?.sessionBound ? (
          <div className="space-y-4" role="alert">
            <p className="text-sm leading-6 text-muted-foreground">
              当前登录来自兼容期旧会话，无法安全绑定本次验证。请重新登录后继续。
            </p>
            <Button
              type="button"
              onClick={() => void logout().then(() => navigate("/login", { replace: true }))}
            >
              重新登录
            </Button>
          </div>
        ) : status.data.isFresh ? (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground" role="status">当前设备的安全验证仍然有效。</p>
            <Button type="button" onClick={() => { onOpenChange(false); onVerified(); }}>继续操作</Button>
          </div>
        ) : (
          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
              verify.mutate();
            }}
          >
            <Tabs value={method} onValueChange={(value) => setMethod(value as RecentAuthMethod)}>
              <TabsList aria-label="安全验证方式">
                {availableMethods.includes("password") ? (
                  <TabsTrigger value="password"><KeyRound className="mr-2 size-4" aria-hidden="true" />密码</TabsTrigger>
                ) : null}
                {availableMethods.includes("email_code") ? (
                  <TabsTrigger value="email_code"><MailCheck className="mr-2 size-4" aria-hidden="true" />邮箱验证码</TabsTrigger>
                ) : null}
              </TabsList>
              <TabsContent value="password" className="space-y-2 pt-2">
                <Label htmlFor="recent-auth-password">当前密码</Label>
                <Input
                  id="recent-auth-password"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  minLength={1}
                  maxLength={128}
                  autoFocus
                />
              </TabsContent>
              <TabsContent value="email_code" className="space-y-3 pt-2">
                <p className="text-sm leading-6 text-muted-foreground">
                  验证码只会发送到当前账号已经验证的校园邮箱；此处不会显示或接受其他邮箱。
                </p>
                <div className="flex gap-2">
                  <div className="min-w-0 flex-1 space-y-2">
                    <Label htmlFor="recent-auth-code">六位验证码</Label>
                    <Input
                      id="recent-auth-code"
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      value={code}
                      onChange={(event) => setCode(event.target.value.replace(/\D/g, "").slice(0, 6))}
                      pattern="[0-9]{6}"
                      maxLength={6}
                    />
                  </div>
                  <Button
                    className="mt-8 shrink-0"
                    type="button"
                    variant="outline"
                    onClick={() => sendCode.mutate()}
                    disabled={sendCode.isPending || resendSeconds > 0}
                  >
                    {sendCode.isPending ? "发送中…" : resendSeconds > 0 ? `${resendSeconds} 秒后重发` : "发送验证码"}
                  </Button>
                </div>
              </TabsContent>
            </Tabs>
            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error instanceof Error ? error.message : "验证失败，请重试"}
              </p>
            ) : null}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={verify.isPending}>取消</Button>
              <Button
                type="submit"
                disabled={verify.isPending || (method === "password" ? password.length === 0 : code.length !== 6)}
              >
                {verify.isPending ? "正在验证…" : "完成验证并继续"}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}

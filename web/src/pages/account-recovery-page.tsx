import { useMutation, useQuery } from "@tanstack/react-query";
import { KeyRound, MailCheck, RotateCcw, ShieldCheck } from "lucide-react";
import * as React from "react";
import { Link, Navigate, useNavigate } from "react-router";
import { toast } from "sonner";

import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { clearRecoveryCredential, readRecoveryCredential, storeRecoveryCredential } from "@/lib/account-recovery";
import { api } from "@/lib/api/endpoints";
import type { RecoveryCredential } from "@/lib/api/types";

function campusEmail(value: string) {
  return value.trim().toLowerCase();
}

function isCampusEmail(value: string) {
  return /^[^@\s]+@tongji\.edu\.cn$/i.test(value.trim());
}

function formatRecoveryWindow(timestamp: number | null) {
  if (!timestamp) return "未设置自动清除期限";
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "long", timeStyle: "short" })
    .format(new Date(timestamp * 1_000));
}

export function AccountRecoveryPage() {
  const navigate = useNavigate();
  const { isAuthenticated } = useAuth();
  const [method, setMethod] = React.useState<"password" | "email">("password");
  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [code, setCode] = React.useState("");
  const [credential, setCredential] = React.useState<RecoveryCredential | null>(() => readRecoveryCredential());
  const [captchaOpen, setCaptchaOpen] = React.useState(false);
  const [cooldown, setCooldown] = React.useState(0);

  React.useEffect(() => {
    if (cooldown <= 0) return undefined;
    const timer = window.setTimeout(() => setCooldown((current) => Math.max(0, current - 1)), 1_000);
    return () => window.clearTimeout(timer);
  }, [cooldown]);

  const inspection = useQuery({
    queryKey: ["account-recovery", credential?.recoveryToken],
    queryFn: () => api.inspectRecovery(credential?.recoveryToken ?? ""),
    enabled: Boolean(credential),
    retry: false,
  });

  React.useEffect(() => {
    if (!inspection.isError) return;
    clearRecoveryCredential();
    setCredential(null);
  }, [inspection.isError]);

  const storeCredential = React.useCallback((nextCredential: RecoveryCredential) => {
    storeRecoveryCredential(nextCredential);
    setCredential(nextCredential);
  }, []);

  const verifyPassword = useMutation({
    mutationFn: () => api.recoveryPassword({ email: campusEmail(email), password }),
    onSuccess: storeCredential,
  });
  const sendCode = useMutation({
    mutationFn: (captchaToken: string) => api.requestEmailCode(campusEmail(email), captchaToken, "recovery"),
    onSuccess: () => {
      setCooldown(60);
      toast.success("如果账号处于可恢复状态，验证码将发送到邮箱");
    },
  });
  const verifyCode = useMutation({
    mutationFn: () => api.recoveryEmailVerify({ email: campusEmail(email), code: code.trim() }),
    onSuccess: storeCredential,
  });
  const reactivate = useMutation({
    mutationFn: () => api.reactivateAccount(credential?.recoveryToken ?? ""),
    onSuccess: () => {
      clearRecoveryCredential();
      setCredential(null);
      toast.success("账号已恢复。旧会话保持失效，请重新登录");
      navigate("/login", { replace: true });
    },
  });

  if (isAuthenticated) return <Navigate to="/" replace />;

  const lifecycle = inspection.data ?? credential?.lifecycle;
  const error = verifyPassword.error ?? sendCode.error ?? verifyCode.error ?? reactivate.error;

  return (
    <div className="mx-auto max-w-2xl py-4 sm:py-10">
      <div className="mb-7">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">Account recovery</p>
        <h1 className="mt-2 text-3xl font-bold tracking-tight">恢复已关闭的账号</h1>
        <p className="mt-3 leading-7 text-muted-foreground">
          此流程只适用于主动停用或仍在 30 天恢复期内的删除申请。验证不会直接登录，也不会恢复任何旧会话。
        </p>
      </div>

      {credential ? (
        <Card>
          <CardHeader>
            <CardTitle><h2 className="flex items-center gap-2"><RotateCcw className="size-5 text-primary" aria-hidden="true" />确认恢复账号</h2></CardTitle>
            <CardDescription>身份验证已通过。恢复后需要使用正常登录流程重新建立会话。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {inspection.isLoading ? <p className="text-sm text-muted-foreground" role="status">正在确认恢复资格…</p> : (
              <dl className="grid gap-3 rounded-lg border bg-muted/20 p-4 text-sm sm:grid-cols-2">
                <div><dt className="text-muted-foreground">账号状态</dt><dd className="mt-1 font-medium">{lifecycle?.state === "deactivated" ? "已停用" : "删除恢复期"}</dd></div>
                <div><dt className="text-muted-foreground">最晚恢复时间</dt><dd className="mt-1 font-medium">{formatRecoveryWindow(lifecycle?.recoverUntil ?? null)}</dd></div>
              </dl>
            )}
            <div className="rounded-lg border border-primary/20 bg-primary/[0.05] p-4 text-sm leading-6 text-muted-foreground">
              恢复会取消未完成的删除任务，并重新允许账号参与社区；此前被撤销的设备会话不会复活。
            </div>
            {error ? <p className="text-sm text-destructive" role="alert">{error instanceof Error ? error.message : "恢复失败，请重试"}</p> : null}
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  clearRecoveryCredential();
                  setCredential(null);
                }}
                disabled={reactivate.isPending}
              >重新验证</Button>
              <Button type="button" onClick={() => reactivate.mutate()} disabled={inspection.isLoading || reactivate.isPending}>
                {reactivate.isPending ? "正在恢复…" : "确认恢复账号"}
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle><h2 className="flex items-center gap-2"><ShieldCheck className="size-5 text-primary" aria-hidden="true" />验证账号所有权</h2></CardTitle>
            <CardDescription>为避免泄露账号状态，不符合恢复条件与凭证错误会使用相同的失败提示。</CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs value={method} onValueChange={(value) => setMethod(value as "password" | "email")}>
              <TabsList className="grid w-full grid-cols-2" aria-label="恢复验证方式">
                <TabsTrigger value="password"><KeyRound className="mr-2 size-4" aria-hidden="true" />密码</TabsTrigger>
                <TabsTrigger value="email"><MailCheck className="mr-2 size-4" aria-hidden="true" />邮箱验证码</TabsTrigger>
              </TabsList>
              <div className="mt-5 space-y-2">
                <Label htmlFor="recovery-email">同济邮箱</Label>
                <Input id="recovery-email" type="email" autoComplete="email" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@tongji.edu.cn" required />
              </div>
              <TabsContent value="password" className="pt-4">
                <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); verifyPassword.mutate(); }}>
                  <div className="space-y-2">
                    <Label htmlFor="recovery-password">当前密码</Label>
                    <Input id="recovery-password" type="password" autoComplete="current-password" minLength={1} maxLength={128} value={password} onChange={(event) => setPassword(event.target.value)} required />
                  </div>
                  <Button className="w-full" type="submit" disabled={!isCampusEmail(email) || !password || verifyPassword.isPending}>
                    {verifyPassword.isPending ? "正在验证…" : "验证恢复资格"}
                  </Button>
                </form>
              </TabsContent>
              <TabsContent value="email" className="pt-4">
                <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); verifyCode.mutate(); }}>
                  <div className="space-y-2">
                    <Label htmlFor="recovery-code">六位恢复验证码</Label>
                    <div className="flex flex-col gap-2 sm:flex-row">
                      <Input id="recovery-code" inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" maxLength={6} value={code} onChange={(event) => setCode(event.target.value.replace(/\D/g, "").slice(0, 6))} required />
                      <Button type="button" variant="secondary" onClick={() => setCaptchaOpen(true)} disabled={!isCampusEmail(email) || cooldown > 0 || sendCode.isPending}>
                        {sendCode.isPending ? "发送中…" : cooldown > 0 ? `${cooldown} 秒后重发` : "发送恢复码"}
                      </Button>
                    </div>
                  </div>
                  <Button className="w-full" type="submit" disabled={!isCampusEmail(email) || code.length !== 6 || verifyCode.isPending}>
                    {verifyCode.isPending ? "正在验证…" : "验证恢复资格"}
                  </Button>
                </form>
              </TabsContent>
            </Tabs>
            {error ? <p className="mt-4 text-sm text-destructive" role="alert">{error instanceof Error ? error.message : "验证失败，请重试"}</p> : null}
            <p className="mt-5 text-center text-sm text-muted-foreground">账号仍处于正常状态？<Button asChild variant="link" className="h-auto px-1"><Link to="/login">返回登录</Link></Button></p>
          </CardContent>
        </Card>
      )}
      <YourTJCaptcha open={captchaOpen} onOpenChange={setCaptchaOpen} onVerified={(token) => { setCaptchaOpen(false); sendCode.mutate(token); }} />
    </div>
  );
}

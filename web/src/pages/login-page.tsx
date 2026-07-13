import { useMutation } from "@tanstack/react-query";
import { Eye, EyeOff, KeyRound, Mail, ShieldCheck, UserPlus } from "lucide-react";
import * as React from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";

type LoginMode = "password" | "code" | "registration";
type CaptchaAction = "code" | "registration" | "password_reset";

function campusEmail(value: string) {
  return value.trim().toLowerCase();
}

function isCampusEmail(value: string) {
  return /^[^@\s]+@tongji\.edu\.cn$/i.test(value.trim());
}

function safeNextPath(value: string | null) {
  return value?.startsWith("/") && !value.startsWith("//") ? value : "/";
}

function useCountdown() {
  const [seconds, setSeconds] = React.useState(0);

  React.useEffect(() => {
    if (seconds <= 0) return;
    const timer = window.setTimeout(() => setSeconds((current) => Math.max(0, current - 1)), 1_000);
    return () => window.clearTimeout(timer);
  }, [seconds]);

  return [seconds, () => setSeconds(60)] as const;
}

function PasswordInput({
  id,
  value,
  onChange,
  autoComplete,
  placeholder,
}: {
  id: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete: string;
  placeholder?: string;
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
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="absolute right-1 top-1/2 size-8 -translate-y-1/2 text-muted-foreground"
        onClick={() => setIsVisible((current) => !current)}
        aria-label={isVisible ? "隐藏密码" : "显示密码"}
      >
        {isVisible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
      </Button>
    </div>
  );
}

export function LoginPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const {
    requestCode,
    verifyEmail,
    loginWithPassword,
    acceptAuthTokens,
    isAuthenticated,
  } = useAuth();
  const [mode, setMode] = React.useState<LoginMode>("password");
  const [captchaAction, setCaptchaAction] = React.useState<CaptchaAction | null>(null);
  const [passwordEmail, setPasswordEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [codeEmail, setCodeEmail] = React.useState("");
  const [code, setCode] = React.useState("");
  const [setPasswordWithCode, setSetPasswordWithCode] = React.useState(false);
  const [codePassword, setCodePassword] = React.useState("");
  const [registrationEmail, setRegistrationEmail] = React.useState("");
  const [registrationCode, setRegistrationCode] = React.useState("");
  const [handle, setHandle] = React.useState("");
  const [registrationPassword, setRegistrationPassword] = React.useState("");
  const [resetOpen, setResetOpen] = React.useState(false);
  const [resetEmail, setResetEmail] = React.useState("");
  const [resetCode, setResetCode] = React.useState("");
  const [newPassword, setNewPassword] = React.useState("");
  const [confirmPassword, setConfirmPassword] = React.useState("");
  const [codeCooldown, startCodeCooldown] = useCountdown();
  const [registrationCooldown, startRegistrationCooldown] = useCountdown();
  const [resetCooldown, startResetCooldown] = useCountdown();
  const destination = safeNextPath(searchParams.get("next"));

  const passwordLogin = useMutation({
    mutationFn: () => loginWithPassword({ email: campusEmail(passwordEmail), password }),
    onSuccess: () => navigate(destination),
    onError: (error) => toast.error(error instanceof Error ? error.message : "登录失败"),
  });
  const emailCodeRequest = useMutation({
    mutationFn: ({ captchaToken, action }: { captchaToken: string; action: "code" | "registration" }) => {
      const email = action === "code" ? codeEmail : registrationEmail;
      return requestCode(campusEmail(email), captchaToken, action === "code" ? "login" : "registration");
    },
    onSuccess: (_result, variables) => {
      if (variables.action === "code") startCodeCooldown();
      else startRegistrationCooldown();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发送失败"),
  });
  const codeLogin = useMutation({
    mutationFn: () => verifyEmail({
      email: campusEmail(codeEmail),
      code: code.trim(),
      purpose: "login",
      password: setPasswordWithCode ? codePassword : undefined,
    }),
    onSuccess: () => navigate(destination),
    onError: (error) => toast.error(error instanceof Error ? error.message : "登录失败"),
  });
  const register = useMutation({
    mutationFn: () => verifyEmail({
      email: campusEmail(registrationEmail),
      code: registrationCode.trim(),
      purpose: "registration",
      handle: handle.trim(),
      password: registrationPassword || undefined,
    }),
    onSuccess: () => navigate(destination),
    onError: (error) => toast.error(error instanceof Error ? error.message : "注册失败"),
  });
  const forgot = useMutation({
    mutationFn: (captchaToken: string) => api.passwordForgot(campusEmail(resetEmail), captchaToken),
    onSuccess: () => {
      startResetCooldown();
      toast.success("如果该账号可重置密码，验证码将发送到邮箱");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "请求失败"),
  });
  const reset = useMutation({
    mutationFn: () => api.passwordReset({
      email: campusEmail(resetEmail),
      code: resetCode.trim(),
      newPassword,
    }),
    onSuccess: async (tokens) => {
      await acceptAuthTokens(tokens);
      toast.success("密码已重置，其他设备均已退出");
      navigate(destination);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "重置失败"),
  });

  React.useEffect(() => {
    if (isAuthenticated) navigate(destination);
  }, [destination, isAuthenticated, navigate]);

  function openCaptcha(action: CaptchaAction) {
    setCaptchaAction(action);
  }

  function handleCaptcha(token: string) {
    const action = captchaAction;
    setCaptchaAction(null);
    if (action === "password_reset") forgot.mutate(token);
    else if (action) emailCodeRequest.mutate({ captchaToken: token, action });
  }

  const handleIsValid = /^[a-z0-9._-]{3,30}$/.test(handle.trim());
  const registrationPasswordIsValid = !registrationPassword || registrationPassword.length >= 8;
  const resetPasswordsMatch = newPassword.length >= 8 && newPassword === confirmPassword;

  return (
    <div className="mx-auto max-w-2xl">
      <PageHeader
        title="登录 YourTJ"
        description="邮箱只用于校园身份与账号安全，不会出现在公开资料中。"
      />
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="size-5 text-primary" aria-hidden="true" />
            选择登录方式
          </CardTitle>
          <CardDescription>密码、验证码登录和注册是彼此独立的流程。</CardDescription>
        </CardHeader>
        <CardContent>
          <Tabs value={mode} onValueChange={(value) => setMode(value as LoginMode)}>
            <TabsList className="grid h-auto w-full grid-cols-3">
              <TabsTrigger value="password">密码登录</TabsTrigger>
              <TabsTrigger value="code">验证码登录</TabsTrigger>
              <TabsTrigger value="registration">注册账号</TabsTrigger>
            </TabsList>

            <TabsContent value="password" className="pt-5">
              {!resetOpen ? (
                <form
                  className="space-y-4"
                  onSubmit={(event) => {
                    event.preventDefault();
                    passwordLogin.mutate();
                  }}
                >
                  <div className="space-y-2">
                    <Label htmlFor="password-email">同济邮箱</Label>
                    <Input
                      id="password-email"
                      type="email"
                      value={passwordEmail}
                      onChange={(event) => setPasswordEmail(event.target.value)}
                      placeholder="name@tongji.edu.cn"
                      autoComplete="email"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <div className="flex items-center justify-between gap-3">
                      <Label htmlFor="password-login">密码</Label>
                      <Button
                        type="button"
                        variant="link"
                        className="h-auto p-0 text-xs"
                        onClick={() => {
                          setResetEmail(passwordEmail);
                          setResetOpen(true);
                        }}
                      >
                        忘记密码？
                      </Button>
                    </div>
                    <PasswordInput
                      id="password-login"
                      value={password}
                      onChange={setPassword}
                      autoComplete="current-password"
                    />
                  </div>
                  <Button
                    type="submit"
                    className="w-full"
                    disabled={!isCampusEmail(passwordEmail) || !password || passwordLogin.isPending}
                  >
                    <KeyRound className="size-4" />
                    {passwordLogin.isPending ? "正在登录" : "使用密码登录"}
                  </Button>
                </form>
              ) : (
                <form
                  className="space-y-4"
                  onSubmit={(event) => {
                    event.preventDefault();
                    reset.mutate();
                  }}
                >
                  <div>
                    <h2 className="font-semibold">重置密码</h2>
                    <p className="mt-1 text-sm text-muted-foreground">
                      为避免泄露账号状态，无论邮箱是否存在，请求结果都会保持一致。
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reset-email">同济邮箱</Label>
                    <div className="flex flex-col gap-2 sm:flex-row">
                      <Input
                        id="reset-email"
                        type="email"
                        value={resetEmail}
                        onChange={(event) => setResetEmail(event.target.value)}
                        placeholder="name@tongji.edu.cn"
                        autoComplete="email"
                        required
                      />
                      <Button
                        type="button"
                        variant="secondary"
                        onClick={() => openCaptcha("password_reset")}
                        disabled={!isCampusEmail(resetEmail) || resetCooldown > 0 || forgot.isPending}
                      >
                        {resetCooldown > 0 ? `${resetCooldown} 秒后重发` : "发送重置码"}
                      </Button>
                    </div>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="reset-code">重置验证码</Label>
                    <Input
                      id="reset-code"
                      value={resetCode}
                      onChange={(event) => setResetCode(event.target.value)}
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      required
                    />
                  </div>
                  <div className="grid gap-4 sm:grid-cols-2">
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
                      <Label htmlFor="confirm-password">确认新密码</Label>
                      <PasswordInput
                        id="confirm-password"
                        value={confirmPassword}
                        onChange={setConfirmPassword}
                        autoComplete="new-password"
                      />
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground">密码需为 8–128 个字符，并避免使用邮箱、姓名或常见组合。</p>
                  <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                    <Button type="button" variant="ghost" onClick={() => setResetOpen(false)}>返回登录</Button>
                    <Button type="submit" disabled={!resetCode.trim() || !resetPasswordsMatch || reset.isPending}>
                      {reset.isPending ? "正在重置" : "重置密码"}
                    </Button>
                  </div>
                </form>
              )}
            </TabsContent>

            <TabsContent value="code" className="pt-5">
              <form
                className="space-y-4"
                onSubmit={(event) => {
                  event.preventDefault();
                  codeLogin.mutate();
                }}
              >
                <div className="space-y-2">
                  <Label htmlFor="code-email">已注册的同济邮箱</Label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Input
                      id="code-email"
                      type="email"
                      value={codeEmail}
                      onChange={(event) => setCodeEmail(event.target.value)}
                      placeholder="name@tongji.edu.cn"
                      autoComplete="email"
                      required
                    />
                    <Button
                      type="button"
                      variant="secondary"
                      onClick={() => openCaptcha("code")}
                      disabled={!isCampusEmail(codeEmail) || codeCooldown > 0 || emailCodeRequest.isPending}
                    >
                      {codeCooldown > 0 ? `${codeCooldown} 秒后重发` : "发送登录码"}
                    </Button>
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="login-code">登录验证码</Label>
                  <Input
                    id="login-code"
                    value={code}
                    onChange={(event) => setCode(event.target.value)}
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    required
                  />
                </div>
                <div className="rounded-lg border bg-muted/30 p-3">
                  <Button
                    type="button"
                    variant="link"
                    className="h-auto p-0 text-sm"
                    onClick={() => {
                      setSetPasswordWithCode((current) => !current);
                      setCodePassword("");
                    }}
                  >
                    {setPasswordWithCode ? "暂不设置密码" : "本账号尚未设置密码？"}
                  </Button>
                  {setPasswordWithCode ? (
                    <div className="mt-3 space-y-2">
                      <Label htmlFor="code-new-password">首次设置密码</Label>
                      <PasswordInput
                        id="code-new-password"
                        value={codePassword}
                        onChange={setCodePassword}
                        autoComplete="new-password"
                      />
                      <p className="text-xs text-muted-foreground">仅未设置过密码的账号会保存此密码；已有密码不会被验证码登录覆盖。</p>
                    </div>
                  ) : null}
                </div>
                <Button
                  type="submit"
                  className="w-full"
                  disabled={
                    !isCampusEmail(codeEmail)
                    || !code.trim()
                    || (setPasswordWithCode && codePassword.length < 8)
                    || codeLogin.isPending
                  }
                >
                  <Mail className="size-4" />
                  {codeLogin.isPending ? "正在验证" : "使用验证码登录"}
                </Button>
              </form>
            </TabsContent>

            <TabsContent value="registration" className="pt-5">
              <form
                className="space-y-4"
                onSubmit={(event) => {
                  event.preventDefault();
                  register.mutate();
                }}
              >
                <div className="space-y-2">
                  <Label htmlFor="registration-email">同济邮箱</Label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Input
                      id="registration-email"
                      type="email"
                      value={registrationEmail}
                      onChange={(event) => setRegistrationEmail(event.target.value)}
                      placeholder="name@tongji.edu.cn"
                      autoComplete="email"
                      required
                    />
                    <Button
                      type="button"
                      variant="secondary"
                      onClick={() => openCaptcha("registration")}
                      disabled={!isCampusEmail(registrationEmail) || registrationCooldown > 0 || emailCodeRequest.isPending}
                    >
                      {registrationCooldown > 0 ? `${registrationCooldown} 秒后重发` : "发送注册码"}
                    </Button>
                  </div>
                </div>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="registration-code">注册验证码</Label>
                    <Input
                      id="registration-code"
                      value={registrationCode}
                      onChange={(event) => setRegistrationCode(event.target.value)}
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="registration-handle">公开 handle</Label>
                    <Input
                      id="registration-handle"
                      value={handle}
                      onChange={(event) => setHandle(event.target.value.toLowerCase())}
                      placeholder="your-handle"
                      autoComplete="username"
                      minLength={3}
                      maxLength={30}
                      pattern="[a-z0-9._-]{3,30}"
                      required
                    />
                    <p className="text-xs text-muted-foreground">3–30 位小写字母、数字、点、短横线或下划线；请勿使用学号或姓名。</p>
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="registration-password">设置密码（推荐）</Label>
                  <PasswordInput
                    id="registration-password"
                    value={registrationPassword}
                    onChange={setRegistrationPassword}
                    autoComplete="new-password"
                    placeholder="也可先使用验证码登录"
                  />
                  <p className="text-xs text-muted-foreground">8–128 个字符；设置后可直接使用密码登录。</p>
                </div>
                <Button
                  type="submit"
                  className="w-full"
                  disabled={
                    !isCampusEmail(registrationEmail)
                    || !registrationCode.trim()
                    || !handleIsValid
                    || !registrationPasswordIsValid
                    || register.isPending
                  }
                >
                  <UserPlus className="size-4" />
                  {register.isPending ? "正在创建账号" : "验证并注册"}
                </Button>
                <p className="text-xs leading-5 text-muted-foreground">
                  注册后会先引导你选择隐私范围并明确接受当前社区规则与隐私说明；邮箱仅用于校园资格、风控和账号恢复。
                </p>
              </form>
            </TabsContent>
          </Tabs>
          <div className="mt-5 border-t pt-4 text-center text-sm text-muted-foreground">
            主动停用或正在删除恢复期？
            <Button asChild variant="link" className="h-auto px-1">
              <Link to="/recover-account">恢复账号</Link>
            </Button>
          </div>
        </CardContent>
      </Card>
      <YourTJCaptcha
        open={captchaAction !== null}
        onOpenChange={(open) => {
          if (!open) setCaptchaAction(null);
        }}
        onVerified={handleCaptcha}
      />
    </div>
  );
}

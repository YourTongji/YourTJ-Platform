import { useMutation } from "@tanstack/react-query";
import { Mail, ShieldCheck } from "lucide-react";
import * as React from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";

export function LoginPage() {
  const navigate = useNavigate();
  const { requestCode, verifyEmail, isAuthenticated } = useAuth();
  const [email, setEmail] = React.useState("");
  const [code, setCode] = React.useState("");
  const [handle, setHandle] = React.useState("");
  const [password, setPassword] = React.useState("");
  const request = useMutation({
    mutationFn: () => requestCode(email),
    onError: (error) => toast.error(error instanceof Error ? error.message : "发送失败"),
  });
  const verify = useMutation({
    mutationFn: () =>
      verifyEmail({
        email,
        code,
        handle: handle || undefined,
        password: password || undefined,
      }),
    onSuccess: () => navigate("/"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "登录失败"),
  });

  React.useEffect(() => {
    if (isAuthenticated) {
      navigate("/");
    }
  }, [isAuthenticated, navigate]);

  return (
    <div className="mx-auto max-w-xl">
      <PageHeader
        eyebrow="Auth"
        title="校园邮箱登录"
        description="使用 @tongji.edu.cn 邮箱验证码登录或注册。邮箱只用于身份与风控，不在公开页面展示。"
      />
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5 text-primary" />
            登录 YourTJ
          </CardTitle>
          <CardDescription>首次登录可填写公开 handle，后续也可在设置里修改。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>同济邮箱</Label>
            <div className="flex gap-2">
              <Input value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@tongji.edu.cn" />
              <Button variant="secondary" onClick={() => request.mutate()} disabled={!email || request.isPending}>
                发送验证码
              </Button>
            </div>
          </div>
          <div className="space-y-2">
            <Label>验证码</Label>
            <Input value={code} onChange={(event) => setCode(event.target.value)} inputMode="numeric" />
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>公开 handle</Label>
              <Input value={handle} onChange={(event) => setHandle(event.target.value)} placeholder="可选" />
            </div>
            <div className="space-y-2">
              <Label>密码</Label>
              <Input type="password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder="可选，首次设置" />
            </div>
          </div>
          <Button className="w-full" onClick={() => verify.mutate()} disabled={!email || !code || verify.isPending}>
            <ShieldCheck className="h-4 w-4" />
            验证并登录
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

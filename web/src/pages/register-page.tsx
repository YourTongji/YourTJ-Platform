import { useMutation } from "@tanstack/react-query";
import { useEffect } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";

function isCampusEmail(value: string) {
  return /^[^@\s]+@tongji\.edu\.cn$/i.test(value.trim());
}

const HANDLE_PATTERN = /^[a-z0-9._-]{3,30}$/;

export function RegisterPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { isAuthenticated } = useAuth();
  const destination = searchParams.get("returnTo")?.startsWith("/") ? searchParams.get("returnTo")! : "/";

  useEffect(() => { if (isAuthenticated) navigate(destination, { replace: true }); }, [isAuthenticated, navigate, destination]);

  const sendCode = useMutation({
    mutationFn: (email: string) => api.requestEmailCode(email, "", "registration"),
    onSuccess: () => toast.success("验证码已发送"),
    onError: () => toast.error("发送失败"),
  });

  const doRegister = useMutation({
    mutationFn: (input: { email: string; code: string; handle: string; password?: string }) =>
      api.register(input),
    onSuccess: () => { toast.success("注册成功"); navigate("/login"); },
    onError: () => toast.error("注册失败"),
  });

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const data = new FormData(e.currentTarget);
    const email = (data.get("email") as string).trim().toLowerCase();
    const handle = (data.get("handle") as string).trim().toLowerCase();
    const password = (data.get("password") as string) || undefined;
    if (!isCampusEmail(email)) { toast.error("请使用 @tongji.edu.cn 邮箱"); return; }
    if (!HANDLE_PATTERN.test(handle)) { toast.error("Handle 需3-30个字符: a-z, 0-9, . _ -"); return; }
    doRegister.mutate({ email, code: data.get("code") as string, handle, password });
  };

  if (isAuthenticated) return null;

  return (
    <div className="mx-auto mt-16 max-w-sm">
      <Card>
        <CardHeader>
          <CardTitle>注册 YourTJ</CardTitle>
          <CardDescription>使用校园邮箱创建新账号</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <Label htmlFor="reg-email">校园邮箱</Label>
              <Input id="reg-email" name="email" type="email" placeholder="xxx@tongji.edu.cn" required />
            </div>
            <div>
              <Label htmlFor="reg-code">验证码</Label>
              <div className="flex gap-2">
                <Input id="reg-code" name="code" placeholder="6位验证码" maxLength={6} required />
                <Button type="button" variant="outline" onClick={() => {
                  const email = (document.getElementById("reg-email") as HTMLInputElement)?.value.trim().toLowerCase();
                  if (!isCampusEmail(email)) { toast.error("请先输入有效的校园邮箱"); return; }
                  sendCode.mutate(email);
                }} disabled={sendCode.isPending}>
                  {sendCode.isPending ? "发送中..." : "发送验证码"}
                </Button>
              </div>
            </div>
            <div>
              <Label htmlFor="reg-handle">公开 Handle</Label>
              <Input id="reg-handle" name="handle" placeholder="例如: walker" maxLength={30} required />
              <p className="text-muted-foreground mt-1 text-xs">3-30个字符: 小写字母、数字、. _ -</p>
            </div>
            <div>
              <Label htmlFor="reg-password">密码（可选）</Label>
              <Input id="reg-password" name="password" type="password" placeholder="不设置可用验证码登录" />
            </div>
            <Button type="submit" className="w-full" disabled={doRegister.isPending}>
              {doRegister.isPending ? "注册中..." : "注册"}
            </Button>
          </form>
          <p className="text-muted-foreground mt-4 text-center text-sm">
            已有账号？<Link to="/login" className="underline">登录</Link>
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

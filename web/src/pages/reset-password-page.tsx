import { useMutation } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api/endpoints";

export function ResetPasswordPage() {
  const navigate = useNavigate();

  const reset = useMutation({
    mutationFn: () => {
      const form = document.forms.namedItem("reset-form")!;
      const data = new FormData(form);
      return api.passwordReset({
        email: (data.get("email") as string).trim().toLowerCase(),
        code: data.get("code") as string,
        newPassword: data.get("password") as string,
      });
    },
    onSuccess: () => {
      toast.success("密码已重置，请用新密码登录");
      navigate("/login?reset=success");
    },
    onError: () => toast.error("重置失败，验证码可能已过期"),
  });

  return (
    <div className="mx-auto mt-16 max-w-sm">
      <Card>
        <CardHeader>
          <CardTitle>重置密码</CardTitle>
          <CardDescription>输入邮箱、验证码和新密码</CardDescription>
        </CardHeader>
        <CardContent>
          <form name="reset-form" onSubmit={(e) => { e.preventDefault(); reset.mutate(); }} className="space-y-4">
            <div>
              <Label htmlFor="rp-email">校园邮箱</Label>
              <Input id="rp-email" name="email" type="email" placeholder="xxx@tongji.edu.cn" required />
            </div>
            <div>
              <Label htmlFor="rp-code">验证码</Label>
              <Input id="rp-code" name="code" placeholder="6位验证码" maxLength={6} required />
            </div>
            <div>
              <Label htmlFor="rp-password">新密码</Label>
              <Input id="rp-password" name="password" type="password" placeholder="至少8位" minLength={8} required />
            </div>
            <Button type="submit" className="w-full" disabled={reset.isPending}>
              {reset.isPending ? "重置中..." : "重置密码"}
            </Button>
          </form>
          <p className="text-muted-foreground mt-4 text-center text-sm">
            <Link to="/login" className="underline">返回登录</Link>
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

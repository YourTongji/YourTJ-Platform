import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api/endpoints";

function isCampusEmail(value: string) {
  return /^[^@\s]+@tongji\.edu\.cn$/i.test(value.trim());
}

export function ForgotPasswordPage() {
  const [sent, setSent] = useState(false);

  const sendCode = useMutation({
    mutationFn: () => {
      const email = (document.getElementById("fp-email") as HTMLInputElement)?.value.trim().toLowerCase();
      if (!isCampusEmail(email)) { toast.error("请使用 @tongji.edu.cn 邮箱"); return Promise.reject(); }
      return api.passwordForgot(email, "");
    },
    onSuccess: () => { setSent(true); toast.success("若该邮箱已注册，已发送重置验证码"); },
    onError: () => toast.error("发送失败，请稍后重试"),
  });

  return (
    <div className="mx-auto mt-16 max-w-sm">
      <Card>
        <CardHeader>
          <CardTitle>忘记密码</CardTitle>
          <CardDescription>输入校园邮箱，若已注册将收到重置验证码</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="fp-email">校园邮箱</Label>
            <Input id="fp-email" type="email" placeholder="xxx@tongji.edu.cn" />
          </div>
          <Button onClick={() => sendCode.mutate()} className="w-full" disabled={sendCode.isPending || sent}>
            {sendCode.isPending ? "发送中..." : sent ? "已发送" : "发送重置验证码"}
          </Button>
          {sent && (
            <p className="text-sm text-center">
              <Link to="/reset-password" className="underline text-primary">下一步：重置密码</Link>
            </p>
          )}
          <p className="text-muted-foreground text-center text-sm">
            记起来了？<Link to="/login" className="underline">返回登录</Link>
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

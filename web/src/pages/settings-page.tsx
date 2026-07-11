import { useMutation } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/common/states";
import { SecuritySettings } from "@/components/settings/security-settings";
import { NotificationSettings } from "@/components/settings/notification-settings";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/context/auth-provider";

export function SettingsPage() {
  const { account, isAuthenticated, updateProfile } = useAuth();
  const [handle, setHandle] = React.useState(account?.handle ?? "");
  const [avatarUrl, setAvatarUrl] = React.useState(account?.avatarUrl ?? "");
  const mutation = useMutation({
    mutationFn: () => updateProfile({ handle: handle || undefined, avatarUrl: avatarUrl || undefined }),
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  React.useEffect(() => {
    setHandle(account?.handle ?? "");
    setAvatarUrl(account?.avatarUrl ?? "");
  }, [account]);


  if (!isAuthenticated) {
    return <EmptyState title="登录后修改设置" />;
  }

  return (
    <div className="max-w-2xl">
      <PageHeader eyebrow="Settings" title="设置" description="管理公开资料和本机偏好。" />
      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>公开资料</CardTitle>
            <CardDescription>邮箱不会公开展示。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="space-y-2">
              <Label>Handle</Label>
              <Input value={handle} onChange={(event) => setHandle(event.target.value)} />
            </div>
            <div className="space-y-2">
              <Label>头像 URL</Label>
              <Input value={avatarUrl} onChange={(event) => setAvatarUrl(event.target.value)} />
            </div>
            <Button onClick={() => mutation.mutate()} disabled={mutation.isPending}>保存</Button>
          </CardContent>
        </Card>

        <NotificationSettings />

        <SecuritySettings />
      </div>
    </div>
  );
}

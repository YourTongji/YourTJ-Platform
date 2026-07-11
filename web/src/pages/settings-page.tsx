import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/common/states";
import { SecuritySettings } from "@/components/settings/security-settings";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";

export function SettingsPage() {
  const { account, isAuthenticated, updateProfile } = useAuth();
  const queryClient = useQueryClient();
  const [handle, setHandle] = React.useState(account?.handle ?? "");
  const [avatarUrl, setAvatarUrl] = React.useState(account?.avatarUrl ?? "");
  const [emailPush, setEmailPush] = React.useState(true);
  const [webPush, setWebPush] = React.useState(true);
  const prefs = useQuery({
    queryKey: ["notification-prefs"],
    queryFn: api.notificationPrefs,
    enabled: isAuthenticated,
  });
  const mutation = useMutation({
    mutationFn: () => updateProfile({ handle: handle || undefined, avatarUrl: avatarUrl || undefined }),
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });
  const savePrefs = useMutation({
    mutationFn: () => api.updateNotificationPrefs({ emailPush, webPush }),
    onSuccess: async () => {
      toast.success("通知偏好已保存");
      await queryClient.invalidateQueries({ queryKey: ["notification-prefs"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  React.useEffect(() => {
    setHandle(account?.handle ?? "");
    setAvatarUrl(account?.avatarUrl ?? "");
  }, [account]);

  React.useEffect(() => {
    const nextPrefs = prefs.data?.prefs;
    if (typeof nextPrefs?.emailPush === "boolean") {
      setEmailPush(nextPrefs.emailPush);
    }
    if (typeof nextPrefs?.webPush === "boolean") {
      setWebPush(nextPrefs.webPush);
    }
  }, [prefs.data]);

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

        <Card>
          <CardHeader>
            <CardTitle>通知偏好</CardTitle>
            <CardDescription>读写 `/me/notification-prefs`，由后端按账号保存。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="font-medium">站内重要通知</p>
                <p className="text-sm text-muted-foreground">回复、订阅和系统消息。</p>
              </div>
              <Switch checked={webPush} onCheckedChange={setWebPush} />
            </div>
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="font-medium">邮件提醒</p>
                <p className="text-sm text-muted-foreground">只用于重要账号与安全通知。</p>
              </div>
              <Switch checked={emailPush} onCheckedChange={setEmailPush} />
            </div>
            <Button variant="outline" onClick={() => savePrefs.mutate()} disabled={savePrefs.isPending}>
              保存通知偏好
            </Button>
          </CardContent>
        </Card>

        <SecuritySettings />
      </div>
    </div>
  );
}

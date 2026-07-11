import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bell, Mail } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api/endpoints";
import type { NotificationPreferences } from "@/lib/api/types";

const defaultPreferences: NotificationPreferences = {
  inApp: {
    replies: true,
    mentions: true,
    quotes: true,
    votes: true,
    badges: true,
    subscriptions: true,
    directMessages: true,
  },
  email: { weeklyDigest: false },
};

const inAppOptions: Array<{
  key: keyof NotificationPreferences["inApp"];
  label: string;
  description: string;
}> = [
  { key: "replies", label: "回复", description: "有人回复你的主题或评论。" },
  { key: "mentions", label: "提及", description: "有人在公开内容中提到你的 handle。" },
  { key: "quotes", label: "引用", description: "有人引用你的评论。" },
  { key: "votes", label: "赞同", description: "你的内容获得新的赞同。" },
  { key: "badges", label: "成就徽章", description: "账号获得新的社区成就。" },
  { key: "subscriptions", label: "订阅更新", description: "你正在关注的主题出现更新。" },
  { key: "directMessages", label: "私信", description: "收到新的私信；会话级静音优先。" },
];

export function NotificationSettings() {
  const queryClient = useQueryClient();
  const [draft, setDraft] = React.useState<NotificationPreferences>(defaultPreferences);
  const preferences = useQuery({
    queryKey: ["notification-prefs"],
    queryFn: api.notificationPrefs,
  });
  const save = useMutation({
    mutationFn: () => api.updateNotificationPrefs(draft),
    onSuccess: async (result) => {
      setDraft(result.prefs);
      toast.success("通知偏好已保存");
      await queryClient.invalidateQueries({ queryKey: ["notification-prefs"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "保存失败"),
  });

  React.useEffect(() => {
    if (preferences.data?.prefs) setDraft(preferences.data.prefs);
  }, [preferences.data]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Bell className="size-5 text-primary" aria-hidden="true" />
          通知偏好
        </CardTitle>
        <CardDescription>按事件和渠道设置可选提醒；账号安全、制裁和内容处置通知始终保留。</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        {preferences.isLoading ? <p role="status" className="text-sm text-muted-foreground">正在加载通知偏好</p> : null}
        {preferences.isError ? (
          <div className="flex flex-wrap items-center justify-between gap-3" role="alert">
            <p className="text-sm text-destructive">通知偏好加载失败，当前没有覆盖服务端设置。</p>
            <Button type="button" variant="outline" size="sm" onClick={() => void preferences.refetch()}>重试</Button>
          </div>
        ) : null}
        <section aria-labelledby="in-app-notification-title">
          <h3 id="in-app-notification-title" className="mb-3 text-sm font-semibold">站内互动</h3>
          <div className="divide-y rounded-lg border">
            {inAppOptions.map((option) => (
              <label key={option.key} className="flex cursor-pointer items-center justify-between gap-4 p-3">
                <span>
                  <span className="block text-sm font-medium">{option.label}</span>
                  <span className="mt-0.5 block text-xs text-muted-foreground">{option.description}</span>
                </span>
                <Switch
                  checked={draft.inApp[option.key]}
                  onCheckedChange={(checked) => setDraft((current) => ({
                    ...current,
                    inApp: { ...current.inApp, [option.key]: checked },
                  }))}
                  aria-label={`站内${option.label}通知`}
                />
              </label>
            ))}
          </div>
        </section>
        <section aria-labelledby="email-notification-title">
          <h3 id="email-notification-title" className="mb-3 flex items-center gap-2 text-sm font-semibold">
            <Mail className="size-4 text-primary" aria-hidden="true" />邮件
          </h3>
          <label className="flex cursor-pointer items-center justify-between gap-4 rounded-lg border p-3">
            <span>
              <span className="block text-sm font-medium">每周社区摘要</span>
              <span className="mt-0.5 block text-xs text-muted-foreground">汇总一周热门主题；验证码和安全邮件不受此开关影响。</span>
            </span>
            <Switch
              checked={draft.email.weeklyDigest}
              onCheckedChange={(weeklyDigest) => setDraft((current) => ({
                ...current,
                email: { weeklyDigest },
              }))}
              aria-label="每周社区摘要邮件"
            />
          </label>
        </section>
        <div className="flex justify-end">
          <Button type="button" onClick={() => save.mutate()} disabled={preferences.isLoading || preferences.isError || save.isPending}>
            {save.isPending ? "正在保存" : "保存通知偏好"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

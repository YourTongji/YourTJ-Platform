import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Info, ShieldAlert } from "lucide-react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Announcement } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

const severityMeta = {
  info: { label: "信息", icon: Info },
  success: { label: "进展", icon: CheckCircle2 },
  warning: { label: "重要提醒", icon: AlertTriangle },
  critical: { label: "紧急", icon: ShieldAlert },
} as const;

function scheduleText(announcement: Announcement) {
  const starts = announcement.startsAt ? formatUnixTime(announcement.startsAt) : "立即生效";
  const ends = announcement.endsAt ? formatUnixTime(announcement.endsAt) : "长期有效";
  return `${starts} — ${ends}`;
}

function AnnouncementCard({ announcement }: { announcement: Announcement }) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const acknowledge = useMutation({
    mutationFn: () => api.recordAnnouncementReceipt(announcement.id, {
      revision: announcement.revision,
      action: "acknowledge",
    }),
    onSuccess: async () => {
      toast.success("已确认公告");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["announcements", "active"] }),
        queryClient.invalidateQueries({ queryKey: ["announcements", "unread"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "确认失败"),
  });
  const meta = severityMeta[announcement.severity];
  const SeverityIcon = meta.icon;
  const acknowledged = Boolean(announcement.receipt?.acknowledgedAt);

  return (
    <Card className={announcement.severity === "critical" ? "border-destructive/40" : undefined}>
      <CardContent className="p-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={announcement.severity === "critical" ? "destructive" : "secondary"}>
                <SeverityIcon className="size-3.5" aria-hidden="true" />
                {meta.label}
              </Badge>
              <Badge variant="outline">版本 {announcement.revision}</Badge>
              {announcement.receipt?.firstSeenAt ? <Badge variant="outline">已查看</Badge> : null}
              {acknowledged ? <Badge>已确认</Badge> : null}
            </div>
            <h2 className="mt-3 text-lg font-semibold">{announcement.title}</h2>
          </div>
          {isAuthenticated && announcement.requiresAck && !acknowledged ? (
            <Button size="sm" disabled={acknowledge.isPending} onClick={() => acknowledge.mutate()}>
              {acknowledge.isPending ? "确认中…" : "我已知晓"}
            </Button>
          ) : null}
        </div>
        {announcement.body ? (
          <p className="mt-4 whitespace-pre-wrap text-sm leading-7 text-foreground/90">{announcement.body}</p>
        ) : null}
        <div className="mt-4 border-t pt-3 text-xs leading-5 text-muted-foreground">
          <p>当前状态：{announcement.effectiveState === "active" ? "生效中" : announcement.effectiveState}</p>
          <p>有效期：{scheduleText(announcement)}</p>
        </div>
      </CardContent>
    </Card>
  );
}
export function AnnouncementsPage() {
  const announcements = useQuery({
    queryKey: ["announcements", "active"],
    queryFn: api.announcements,
  });

  return (
    <div>
      <PageHeader
        eyebrow="Platform announcements"
        title="社区公告"
        description="这里展示当前对你生效的平台公告、版本、有效期与确认状态。"
      />
      {announcements.isLoading ? (
        <LoadingState label="加载社区公告" />
      ) : announcements.isError ? (
        <ErrorState
          title="公告加载失败"
          error={announcements.error}
          onRetry={() => void announcements.refetch()}
        />
      ) : (announcements.data ?? []).length === 0 ? (
        <EmptyState title="当前没有生效中的公告" description="新公告发布后会显示在这里。" />
      ) : (
        <div className="space-y-4">
          {announcements.data?.map((announcement) => (
            <AnnouncementCard key={`${announcement.id}:${announcement.revision}`} announcement={announcement} />
          ))}
        </div>
      )}
    </div>
  );
}

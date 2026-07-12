import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Laptop, LogOut, Monitor, Smartphone, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { api } from "@/lib/api/endpoints";
import type { DeviceSession } from "@/lib/api/types";
import { formatRelativeTime, formatUnixTime } from "@/lib/format";

const DEVICE_ICON_MAP: Array<[RegExp, typeof Monitor]> = [
  [/mobile|phone|iphone|ipod|android|ios/i, Smartphone],
  [/tablet|ipad/i, Smartphone],
  [/windows|mac(os)?|linux|ubuntu|chrome(os)?|desktop/i, Monitor],
];

function pickDeviceIcon(label?: string | null) {
  if (!label) return Laptop;
  for (const [pattern, Icon] of DEVICE_ICON_MAP) {
    if (pattern.test(label)) return Icon;
  }
  return Laptop;
}

function formatDeviceDisplay(label?: string | null) {
  if (!label) return "未知设备";
  return label;
}

function SessionCard({
  session,
  onRevoke,
  revokingId,
}: {
  session: DeviceSession;
  onRevoke: (id: string) => void;
  revokingId: string | null;
}) {
  const DeviceIcon = pickDeviceIcon(session.deviceLabel);
  const isRevoking = revokingId === session.id;

  return (
    <Card className={session.isCurrent ? "border-primary/50" : undefined}>
      <CardContent className="flex items-start gap-4 p-4">
        <div className="rounded-md bg-secondary p-2 text-primary" aria-hidden="true">
          <DeviceIcon className="size-5 shrink-0" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-medium">{formatDeviceDisplay(session.deviceLabel)}</p>
            {session.isCurrent ? <Badge variant="secondary">当前会话</Badge> : null}
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            最近活动：{formatRelativeTime(session.lastUsedAt)}
          </p>
          <p className="text-xs text-muted-foreground">
            创建于 {formatUnixTime(session.createdAt)}
          </p>
        </div>
        {!session.isCurrent ? (
          <Button
            variant="ghost"
            size="icon"
            aria-label="撤销此会话"
            disabled={isRevoking}
            onClick={() => onRevoke(session.id)}
          >
            <Trash2 className="size-4" />
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
}

export function SettingsSessionsPage() {
  const queryClient = useQueryClient();

  const sessionsQuery = useQuery({
    queryKey: ["sessions"],
    queryFn: () => api.sessions(),
  });

  const revokeMutation = useMutation({
    mutationFn: (id: string) => api.revokeSession(id),
    onSuccess: async () => {
      toast.success("会话已撤销");
      await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "撤销失败"),
  });

  const revokeOthersMutation = useMutation({
    mutationFn: () => api.revokeOtherSessions(),
    onSuccess: async () => {
      toast.success("已撤销其他所有会话");
      await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  const sessions: DeviceSession[] = sessionsQuery.data?.items ?? [];
  const revokingId = revokeMutation.isPending
    ? (revokeMutation.variables ?? null)
    : null;

  return (
    <div className="mx-auto max-w-lg space-y-6">
      <PageHeader
        eyebrow="Sessions"
        title="设备与会话管理"
        description="查看和管理登录设备与活跃会话。"
      />

      {sessionsQuery.isLoading ? (
        <LoadingState label="加载会话列表" />
      ) : sessionsQuery.isError ? (
        <ErrorState
          title="加载失败"
          error={sessionsQuery.error}
          onRetry={() => void sessionsQuery.refetch()}
        />
      ) : sessions.length === 0 ? (
        <EmptyState title="没有活跃会话" description="在其他设备登录后将显示在这里。" />
      ) : (
        <div className="space-y-3">
          {sessions.length > 1 ? (
            <div className="flex justify-end">
              <Button
                variant="outline"
                size="sm"
                disabled={revokeOthersMutation.isPending}
                onClick={() => revokeOthersMutation.mutate()}
              >
                <LogOut className="size-4" />
                {revokeOthersMutation.isPending ? "撤销中…" : "撤销其他所有会话"}
              </Button>
            </div>
          ) : null}

          {sessions.map((session) => (
            <SessionCard
              key={session.id}
              session={session}
              onRevoke={(id) => revokeMutation.mutate(id)}
              revokingId={revokingId}
            />
          ))}
        </div>
      )}
    </div>
  );
}

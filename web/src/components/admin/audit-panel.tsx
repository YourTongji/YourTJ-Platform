import { useQuery } from "@tanstack/react-query";
import { FileClock, Search } from "lucide-react";
import * as React from "react";

import { AdminSectionHeader, PaginationControls } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

interface AuditFilters {
  actorId: string;
  action: string;
  targetType: string;
}

const emptyFilters: AuditFilters = { actorId: "", action: "", targetType: "" };

export function AuditPanel() {
  const [draft, setDraft] = React.useState<AuditFilters>(emptyFilters);
  const [filters, setFilters] = React.useState<AuditFilters>(emptyFilters);
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const cursor = cursorStack.at(-1);
  const events = useQuery({
    queryKey: ["admin", "audit", filters, cursor],
    queryFn: () => api.adminAuditEvents({
      actorId: filters.actorId || undefined,
      action: filters.action || undefined,
      targetType: filters.targetType || undefined,
      cursor,
    }),
  });

  function applyFilters(next: AuditFilters) {
    setFilters(next);
    setCursorStack([null]);
  }

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="治理审计"
        description="只读查看账号、系统和服务执行的治理事件。一般审计列表不会暴露完整请求体、校园邮箱或未举报私信。"
      />
      <Card>
        <CardContent className="p-4">
          <form
            className="grid gap-3 md:grid-cols-[1fr_1fr_1fr_auto] md:items-end"
            onSubmit={(event) => { event.preventDefault(); applyFilters(draft); }}
          >
            <div className="space-y-2">
              <Label htmlFor="audit-actor">操作人 ID</Label>
              <Input id="audit-actor" value={draft.actorId} onChange={(event) => setDraft((value) => ({ ...value, actorId: event.target.value }))} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="audit-action">动作</Label>
              <Input id="audit-action" value={draft.action} onChange={(event) => setDraft((value) => ({ ...value, action: event.target.value }))} placeholder="如 user.suspend" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="audit-target">目标类型</Label>
              <Input id="audit-target" value={draft.targetType} onChange={(event) => setDraft((value) => ({ ...value, targetType: event.target.value }))} placeholder="如 account" />
            </div>
            <div className="flex gap-2">
              <Button type="submit" variant="outline"><Search className="size-4" />筛选</Button>
              <Button type="button" variant="ghost" onClick={() => { setDraft(emptyFilters); applyFilters(emptyFilters); }}>清除</Button>
            </div>
          </form>
        </CardContent>
      </Card>

      {events.isLoading ? (
        <LoadingState label="加载审计事件" />
      ) : events.isError ? (
        <ErrorState title="审计事件加载失败" error={events.error} onRetry={() => void events.refetch()} />
      ) : (events.data?.items ?? []).length === 0 ? (
        <EmptyState title="没有符合条件的审计事件" />
      ) : (
        <div className="space-y-3">
          {events.data?.items?.map((event) => (
            <Card key={event.id} className="rounded-xl">
              <CardContent className="grid gap-3 p-4 md:grid-cols-[minmax(0,1fr)_auto]">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <FileClock className="size-4 text-primary" aria-hidden="true" />
                    <span className="font-medium">{event.action}</span>
                    <Badge variant="outline">{event.actorKind}</Badge>
                    <Badge variant="secondary">{event.targetType}</Badge>
                  </div>
                  <p className="mt-2 text-sm">
                    操作人 {event.actorHandle ?? event.actorId ?? event.actorKind} · 目标 {event.targetId}
                  </p>
                  {event.reason ? <p className="mt-1 text-sm leading-6 text-muted-foreground">原因：{event.reason}</p> : null}
                  {event.metadata ? <p className="mt-1 text-xs text-muted-foreground">包含受约束的结构化元数据</p> : null}
                </div>
                <p className="text-xs text-muted-foreground md:text-right">{formatUnixTime(event.createdAt)}</p>
              </CardContent>
            </Card>
          ))}
          <PaginationControls
            hasPrevious={cursorStack.length > 1}
            hasMore={Boolean(events.data?.hasMore && events.data.nextCursor)}
            onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
            onNext={() => events.data?.nextCursor && setCursorStack((items) => [...items, events.data?.nextCursor ?? null])}
          />
        </div>
      )}
    </div>
  );
}


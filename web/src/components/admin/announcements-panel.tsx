import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, History, Megaphone, Pencil, Plus } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { AdminSectionHeader, PaginationControls, ReasonDialog } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api/endpoints";
import type { Announcement, AnnouncementCreateInput, AnnouncementUpdateInput } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

type EditableAnnouncementStatus = Exclude<Announcement["status"], "archived">;

function localDateTime(timestamp?: number | null) {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function unixDateTime(value: string) {
  if (!value) return null;
  const timestamp = new Date(value).getTime();
  return Number.isFinite(timestamp) ? Math.floor(timestamp / 1000) : null;
}

function AnnouncementEditor({
  open,
  item,
  onOpenChange,
}: {
  open: boolean;
  item: Announcement | null;
  onOpenChange: (open: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const [title, setTitle] = React.useState("");
  const [body, setBody] = React.useState("");
  const [status, setStatus] = React.useState<EditableAnnouncementStatus>("draft");
  const [presentation, setPresentation] = React.useState<Announcement["presentation"]>("card");
  const [severity, setSeverity] = React.useState<Announcement["severity"]>("info");
  const [audience, setAudience] = React.useState<Announcement["audience"]>("all");
  const [priority, setPriority] = React.useState(0);
  const [startsAt, setStartsAt] = React.useState("");
  const [endsAt, setEndsAt] = React.useState("");
  const [requiresAck, setRequiresAck] = React.useState(false);
  const [bumpRevision, setBumpRevision] = React.useState(false);
  const [reason, setReason] = React.useState("");

  React.useEffect(() => {
    if (!open) return;
    setTitle(item?.title ?? "");
    setBody(item?.body ?? "");
    setStatus(item?.status === "archived" ? "draft" : item?.status ?? "draft");
    setPresentation(item?.presentation ?? "card");
    setSeverity(item?.severity ?? "info");
    setAudience(item?.audience ?? "all");
    setPriority(item?.priority ?? 0);
    setStartsAt(localDateTime(item?.startsAt));
    setEndsAt(localDateTime(item?.endsAt));
    setRequiresAck(item?.requiresAck ?? false);
    setBumpRevision(false);
    setReason("");
  }, [item, open]);

  const save = useMutation({
    mutationFn: async () => {
      const common: AnnouncementCreateInput = {
        title: title.trim(),
        body: body.trim() || null,
        status,
        presentation,
        severity,
        priority,
        audience,
        requiresAck,
        startsAt: unixDateTime(startsAt),
        endsAt: unixDateTime(endsAt),
        reason: reason.trim(),
      };
      if (!item) return api.createAdminAnnouncement(common);
      const update: AnnouncementUpdateInput = {
        ...common,
        expectedVersion: item.version,
        bumpRevision,
      };
      return api.updateAdminAnnouncement(item.id, update);
    },
    onSuccess: async () => {
      toast.success(item ? "公告已更新" : "公告已创建");
      onOpenChange(false);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] }),
        queryClient.invalidateQueries({ queryKey: ["announcements"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "公告保存失败"),
  });
  const isValid = title.trim().length > 0
    && reason.trim().length >= 3
    && (status !== "scheduled" || Boolean(startsAt));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{item ? "编辑公告" : "创建公告"}</DialogTitle>
          <DialogDescription>
            排期与受众由服务端强制执行。只有勾选“重新展示”才会提升 receipt revision。
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="announcement-title">标题</Label>
            <Input id="announcement-title" value={title} onChange={(event) => setTitle(event.target.value)} maxLength={200} />
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="announcement-body">正文</Label>
            <Textarea id="announcement-body" value={body} onChange={(event) => setBody(event.target.value)} maxLength={20000} className="min-h-36" />
          </div>
          <SelectField id="announcement-status" label="状态" value={status} onChange={(value) => setStatus(value as EditableAnnouncementStatus)} options={[
            ["draft", "草稿"], ["scheduled", "已排期"], ["published", "立即发布"],
          ]} />
          <SelectField id="announcement-audience" label="受众" value={audience} onChange={(value) => setAudience(value as Announcement["audience"])} options={[
            ["all", "所有访客"], ["authenticated", "已登录用户"], ["staff", "社区职员"],
          ]} />
          <SelectField id="announcement-severity" label="严重度" value={severity} onChange={(value) => setSeverity(value as Announcement["severity"])} options={[
            ["info", "信息"], ["success", "进展"], ["warning", "重要提醒"], ["critical", "紧急"],
          ]} />
          <SelectField id="announcement-presentation" label="持续展示" value={presentation} onChange={(value) => setPresentation(value as Announcement["presentation"])} options={[
            ["card", "公告卡片"], ["banner", "横幅"],
          ]} />
          <div className="space-y-2">
            <Label htmlFor="announcement-start">开始时间</Label>
            <Input id="announcement-start" type="datetime-local" value={startsAt} onChange={(event) => setStartsAt(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="announcement-end">结束时间</Label>
            <Input id="announcement-end" type="datetime-local" value={endsAt} onChange={(event) => setEndsAt(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="announcement-priority">优先级（-1000 到 1000）</Label>
            <Input id="announcement-priority" type="number" min={-1000} max={1000} value={priority} onChange={(event) => setPriority(Number(event.target.value))} />
          </div>
          <div className="space-y-3 rounded-lg border p-3">
            <label className="flex items-center justify-between gap-3 text-sm" htmlFor="announcement-ack">
              要求明确确认
              <Switch id="announcement-ack" checked={requiresAck} onCheckedChange={setRequiresAck} />
            </label>
            {item ? (
              <label className="flex items-center justify-between gap-3 text-sm" htmlFor="announcement-bump">
                作为新 revision 重新展示
                <Switch id="announcement-bump" checked={bumpRevision} onCheckedChange={setBumpRevision} />
              </label>
            ) : null}
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="announcement-reason">操作原因</Label>
            <Textarea id="announcement-reason" value={reason} onChange={(event) => setReason(event.target.value)} maxLength={500} placeholder="说明发布、排期或修订依据" />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={save.isPending}>取消</Button>
          <Button type="button" onClick={() => save.mutate()} disabled={!isValid || save.isPending}>
            {save.isPending ? "正在保存…" : "保存公告"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SelectField({ id, label, value, onChange, options }: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<[string, string]>;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <select id={id} value={value} onChange={(event) => onChange(event.target.value)} className="h-9 w-full rounded-md border bg-background px-3 text-sm">
        {options.map(([optionValue, optionLabel]) => <option key={optionValue} value={optionValue}>{optionLabel}</option>)}
      </select>
    </div>
  );
}

function RevisionHistory({ item, onClose }: { item: Announcement | null; onClose: () => void }) {
  const revisions = useQuery({
    queryKey: ["admin", "announcements", item?.id, "revisions"],
    queryFn: () => api.adminAnnouncementRevisions(item?.id ?? ""),
    enabled: Boolean(item),
  });
  return (
    <Dialog open={Boolean(item)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>公告变更历史</DialogTitle>
          <DialogDescription>每次 mutation 都追加 version；只有要求重新展示时 receipt revision 才会提升。</DialogDescription>
        </DialogHeader>
        {revisions.isLoading ? <LoadingState label="加载修订历史" /> : revisions.isError ? (
          <ErrorState error={revisions.error} onRetry={() => void revisions.refetch()} />
        ) : (
          <div className="max-h-[50vh] space-y-2 overflow-y-auto">
            {revisions.data?.items?.map((revision) => (
              <div key={revision.version} className="rounded-lg border p-3 text-sm">
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary">version {revision.version}</Badge>
                  <Badge variant="outline">revision {revision.revision}</Badge>
                  <Badge variant="outline">{revision.status}</Badge>
                </div>
                <p className="mt-2 font-medium">{revision.title}</p>
                <p className="mt-1 text-xs text-muted-foreground">{formatUnixTime(revision.createdAt)}</p>
              </div>
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

export function AnnouncementsPanel() {
  const queryClient = useQueryClient();
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [editorOpen, setEditorOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<Announcement | null>(null);
  const [archiving, setArchiving] = React.useState<Announcement | null>(null);
  const [history, setHistory] = React.useState<Announcement | null>(null);
  const cursor = cursorStack.at(-1);
  const announcements = useQuery({
    queryKey: ["admin", "announcements", cursor],
    queryFn: () => api.adminAnnouncements(cursor),
  });
  const archive = useMutation({
    mutationFn: ({ item, reason }: { item: Announcement; reason: string }) => api.archiveAdminAnnouncement(item.id, {
      expectedVersion: item.version,
      reason,
    }),
    onSuccess: async () => {
      toast.success("公告已归档");
      setArchiving(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] }),
        queryClient.invalidateQueries({ queryKey: ["announcements"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "归档失败"),
  });

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="社区公告"
        description="管理草稿、排期、受众、重要程度、revision 与用户 receipt；归档保留历史和审计。"
        actions={(
          <Button type="button" size="sm" onClick={() => { setEditing(null); setEditorOpen(true); }}>
            <Plus className="size-4" />创建公告
          </Button>
        )}
      />
      {announcements.isLoading ? <LoadingState label="加载公告" /> : announcements.isError ? (
        <ErrorState title="公告加载失败" error={announcements.error} onRetry={() => void announcements.refetch()} />
      ) : (announcements.data?.items ?? []).length === 0 ? (
        <EmptyState title="还没有公告" description="先创建草稿，再安排受众与发布时间。" />
      ) : (
        <div className="space-y-3">
          {announcements.data?.items?.map((item) => (
            <Card key={item.id} className="rounded-xl">
              <CardContent className="flex flex-col gap-4 p-4 lg:flex-row lg:items-start lg:justify-between">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <Megaphone className="size-4 shrink-0 text-primary" aria-hidden="true" />
                    <h3 className="font-medium">{item.title}</h3>
                    <Badge variant="secondary">{item.effectiveState}</Badge>
                    <Badge variant="outline">{item.audience}</Badge>
                    <Badge variant="outline">rev {item.revision} / v{item.version}</Badge>
                  </div>
                  <p className="mt-2 line-clamp-3 whitespace-pre-wrap text-sm leading-6 text-muted-foreground">{item.body || "无正文"}</p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    已查看 {item.receiptSummary?.seenCount ?? 0} · 已确认 {item.receiptSummary?.acknowledgedCount ?? 0} · 更新于 {formatUnixTime(item.updatedAt)}
                  </p>
                </div>
                <div className="flex shrink-0 flex-wrap gap-2">
                  <Button type="button" variant="outline" size="sm" onClick={() => setHistory(item)}><History className="size-4" />历史</Button>
                  {item.status !== "archived" ? (
                    <>
                      <Button type="button" variant="outline" size="sm" onClick={() => { setEditing(item); setEditorOpen(true); }}><Pencil className="size-4" />编辑</Button>
                      <Button type="button" variant="destructive" size="sm" onClick={() => setArchiving(item)}><Archive className="size-4" />归档</Button>
                    </>
                  ) : null}
                </div>
              </CardContent>
            </Card>
          ))}
          <PaginationControls
            hasPrevious={cursorStack.length > 1}
            hasMore={Boolean(announcements.data?.hasMore && announcements.data.nextCursor)}
            onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
            onNext={() => announcements.data?.nextCursor && setCursorStack((items) => [...items, announcements.data?.nextCursor ?? null])}
          />
        </div>
      )}
      <AnnouncementEditor open={editorOpen} item={editing} onOpenChange={setEditorOpen} />
      <RevisionHistory item={history} onClose={() => setHistory(null)} />
      <ReasonDialog
        open={Boolean(archiving)}
        onOpenChange={(open) => !open && setArchiving(null)}
        title={`归档公告“${archiving?.title ?? ""}”`}
        description="公告会停止展示，但 revision、receipt 和审计历史都会保留。"
        confirmLabel="确认归档"
        destructive
        isPending={archive.isPending}
        onConfirm={(reason) => archiving && archive.mutate({ item: archiving, reason })}
      />
    </div>
  );
}

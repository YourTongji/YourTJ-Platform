import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Image, Pencil, Plus, RectangleHorizontal } from "lucide-react";
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
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api/endpoints";
import type { Promotion, PromotionCreateInput, PromotionUpdateInput } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

type EditablePromotionStatus = Exclude<Promotion["status"], "archived">;

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

function PromotionEditor({ open, item, onOpenChange }: {
  open: boolean;
  item: Promotion | null;
  onOpenChange: (open: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const [placement, setPlacement] = React.useState<Promotion["placement"]>("home-left-primary");
  const [title, setTitle] = React.useState("");
  const [body, setBody] = React.useState("");
  const [ctaLabel, setCtaLabel] = React.useState("");
  const [targetUrl, setTargetUrl] = React.useState("");
  const [assetId, setAssetId] = React.useState("");
  const [status, setStatus] = React.useState<EditablePromotionStatus>("draft");
  const [priority, setPriority] = React.useState(0);
  const [audience, setAudience] = React.useState<Promotion["audience"]>("all");
  const [startsAt, setStartsAt] = React.useState("");
  const [endsAt, setEndsAt] = React.useState("");
  const [reason, setReason] = React.useState("");

  React.useEffect(() => {
    if (!open) return;
    setPlacement(item?.placement ?? "home-left-primary");
    setTitle(item?.title ?? "");
    setBody(item?.body ?? "");
    setCtaLabel(item?.ctaLabel ?? "");
    setTargetUrl(item?.targetUrl ?? "");
    setAssetId(item?.assetId ?? "");
    setStatus(item?.status === "archived" ? "draft" : item?.status ?? "draft");
    setPriority(item?.priority ?? 0);
    setAudience(item?.audience ?? "all");
    setStartsAt(localDateTime(item?.startsAt));
    setEndsAt(localDateTime(item?.endsAt));
    setReason("");
  }, [item, open]);

  const save = useMutation({
    mutationFn: async () => {
      const common: PromotionCreateInput = {
        placement,
        title: title.trim(),
        body: body.trim() || null,
        ctaLabel: ctaLabel.trim() || null,
        targetUrl: targetUrl.trim(),
        assetId: assetId.trim() || null,
        status,
        priority,
        audience,
        startsAt: unixDateTime(startsAt),
        endsAt: unixDateTime(endsAt),
        reason: reason.trim(),
      };
      if (!item) return api.createAdminPromotion(common);
      const update: PromotionUpdateInput = { ...common, expectedVersion: item.version };
      return api.updateAdminPromotion(item.id, update);
    },
    onSuccess: async () => {
      toast.success(item ? "推广已更新" : "推广已创建");
      onOpenChange(false);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "promotions"] }),
        queryClient.invalidateQueries({ queryKey: ["promotions"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "推广保存失败"),
  });
  const isValid = title.trim().length > 0
    && targetUrl.startsWith("/")
    && !targetUrl.startsWith("//")
    && reason.trim().length >= 3
    && (status !== "scheduled" || Boolean(startsAt));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{item ? "编辑推广" : "创建推广"}</DialogTitle>
          <DialogDescription>
            第一阶段仅允许自营站内路径。素材只能填写当前管理员拥有且审核为 clean 的图片 asset id，不能粘贴图片 URL。
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 sm:grid-cols-2">
          <SelectField id="promotion-placement" label="位置" value={placement} onChange={(value) => setPlacement(value as Promotion["placement"])} options={[
            ["home-left-primary", "首页左侧主位"], ["home-left-secondary", "首页左侧次位"],
          ]} />
          <SelectField id="promotion-status" label="状态" value={status} onChange={(value) => setStatus(value as EditablePromotionStatus)} options={[
            ["draft", "草稿"], ["scheduled", "已排期"], ["published", "立即发布"], ["paused", "暂停"],
          ]} />
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="promotion-title">标题</Label>
            <Input id="promotion-title" value={title} onChange={(event) => setTitle(event.target.value)} maxLength={120} />
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="promotion-body">正文</Label>
            <Textarea id="promotion-body" value={body} onChange={(event) => setBody(event.target.value)} maxLength={500} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="promotion-cta">行动文字</Label>
            <Input id="promotion-cta" value={ctaLabel} onChange={(event) => setCtaLabel(event.target.value)} maxLength={40} placeholder="了解更多" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="promotion-target">站内目标路径</Label>
            <Input id="promotion-target" value={targetUrl} onChange={(event) => setTargetUrl(event.target.value)} maxLength={2048} placeholder="/forum/threads/42" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="promotion-asset">图片 asset id（可选）</Label>
            <Input id="promotion-asset" inputMode="numeric" value={assetId} onChange={(event) => setAssetId(event.target.value)} placeholder="例如 18" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="promotion-priority">排序优先级</Label>
            <Input id="promotion-priority" type="number" min={-1000} max={1000} value={priority} onChange={(event) => setPriority(Number(event.target.value))} />
          </div>
          <SelectField id="promotion-audience" label="受众" value={audience} onChange={(value) => setAudience(value as Promotion["audience"])} options={[
            ["all", "所有访客"], ["authenticated", "已登录用户"], ["staff", "社区职员"],
          ]} />
          <div />
          <div className="space-y-2">
            <Label htmlFor="promotion-start">开始时间</Label>
            <Input id="promotion-start" type="datetime-local" value={startsAt} onChange={(event) => setStartsAt(event.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="promotion-end">结束时间</Label>
            <Input id="promotion-end" type="datetime-local" value={endsAt} onChange={(event) => setEndsAt(event.target.value)} />
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="promotion-reason">操作原因</Label>
            <Textarea id="promotion-reason" value={reason} onChange={(event) => setReason(event.target.value)} maxLength={500} placeholder="说明素材、目标与排期依据" />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={save.isPending}>取消</Button>
          <Button type="button" onClick={() => save.mutate()} disabled={!isValid || save.isPending}>{save.isPending ? "正在保存…" : "保存推广"}</Button>
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

export function PromotionsPanel() {
  const queryClient = useQueryClient();
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [editorOpen, setEditorOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<Promotion | null>(null);
  const [archiving, setArchiving] = React.useState<Promotion | null>(null);
  const cursor = cursorStack.at(-1);
  const promotions = useQuery({
    queryKey: ["admin", "promotions", cursor],
    queryFn: () => api.adminPromotions(cursor),
  });
  const archive = useMutation({
    mutationFn: ({ item, reason }: { item: Promotion; reason: string }) => api.archiveAdminPromotion(item.id, {
      expectedVersion: item.version,
      reason,
    }),
    onSuccess: async () => {
      toast.success("推广已归档");
      setArchiving(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "promotions"] }),
        queryClient.invalidateQueries({ queryKey: ["promotions"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "归档失败"),
  });

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="首页推广"
        description="维护自营推荐的站内目标、clean asset、位置、受众、排期和稳定排序；不接受外链图片。"
        actions={<Button type="button" size="sm" onClick={() => { setEditing(null); setEditorOpen(true); }}><Plus className="size-4" />创建推广</Button>}
      />
      {promotions.isLoading ? <LoadingState label="加载推广" /> : promotions.isError ? (
        <ErrorState title="推广加载失败" error={promotions.error} onRetry={() => void promotions.refetch()} />
      ) : (promotions.data?.items ?? []).length === 0 ? (
        <EmptyState title="还没有推广" description="没有有效推广时，左侧不会显示伪造占位卡。" />
      ) : (
        <div className="space-y-3">
          {promotions.data?.items?.map((item) => (
            <Card key={item.id} className="rounded-xl">
              <CardContent className="flex flex-col gap-4 p-4 lg:flex-row lg:items-start lg:justify-between">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <RectangleHorizontal className="size-4 text-primary" aria-hidden="true" />
                    <h3 className="font-medium">{item.title}</h3>
                    <Badge variant="secondary">{item.effectiveState}</Badge>
                    <Badge variant="outline">{item.placement}</Badge>
                    <Badge variant="outline">优先级 {item.priority}</Badge>
                  </div>
                  {item.body ? <p className="mt-2 text-sm text-muted-foreground">{item.body}</p> : null}
                  <p className="mt-2 text-xs text-muted-foreground">目标 {item.targetUrl} · 受众 {item.audience} · v{item.version} · {formatUnixTime(item.updatedAt)}</p>
                  {item.assetId ? <p className="mt-1 flex items-center gap-1 text-xs text-muted-foreground"><Image className="size-3" />clean asset #{item.assetId}</p> : null}
                </div>
                <div className="flex shrink-0 gap-2">
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
            hasMore={Boolean(promotions.data?.hasMore && promotions.data.nextCursor)}
            onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
            onNext={() => promotions.data?.nextCursor && setCursorStack((items) => [...items, promotions.data?.nextCursor ?? null])}
          />
        </div>
      )}
      <PromotionEditor open={editorOpen} item={editing} onOpenChange={setEditorOpen} />
      <ReasonDialog
        open={Boolean(archiving)}
        onOpenChange={(open) => !open && setArchiving(null)}
        title={`归档推广“${archiving?.title ?? ""}”`}
        description="推广会停止返回给用户，但状态和审计历史保留。"
        confirmLabel="确认归档"
        destructive
        isPending={archive.isPending}
        onConfirm={(reason) => archiving && archive.mutate({ item: archiving, reason })}
      />
    </div>
  );
}

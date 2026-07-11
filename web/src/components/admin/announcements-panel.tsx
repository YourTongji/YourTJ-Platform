import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Megaphone, Pencil, Plus, Trash2 } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { AdminSectionHeader, PaginationControls, ReasonDialog } from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
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
import type { Announcement, AnnouncementInput } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

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
  const [reason, setReason] = React.useState("");

  React.useEffect(() => {
    if (open) {
      setTitle(item?.title ?? "");
      setBody(item?.body ?? "");
      setReason("");
    }
  }, [item, open]);

  const save = useMutation({
    mutationFn: (input: AnnouncementInput) => item
      ? api.updateAdminAnnouncement(item.id, input)
      : api.createAdminAnnouncement(input),
    onSuccess: async () => {
      toast.success(item ? "公告已更新" : "公告已发布");
      onOpenChange(false);
      await queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] });
      await queryClient.invalidateQueries({ queryKey: ["home", "announcements"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "公告保存失败"),
  });
  const isValid = title.trim().length > 0 && reason.trim().length >= 3;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{item ? "编辑公告" : "发布公告"}</DialogTitle>
          <DialogDescription>公告会显示在社区首页。发布与编辑都需要填写审计原因。</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="announcement-title">标题</Label>
            <Input id="announcement-title" value={title} onChange={(event) => setTitle(event.target.value)} maxLength={200} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="announcement-body">正文</Label>
            <Textarea id="announcement-body" value={body} onChange={(event) => setBody(event.target.value)} maxLength={20000} className="min-h-40" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="announcement-reason">操作原因</Label>
            <Textarea id="announcement-reason" value={reason} onChange={(event) => setReason(event.target.value)} maxLength={500} placeholder="说明发布或修改依据" />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={save.isPending}>取消</Button>
          <Button
            type="button"
            onClick={() => save.mutate({ title: title.trim(), body: body.trim() || null, reason: reason.trim() })}
            disabled={!isValid || save.isPending}
          >
            {save.isPending ? "正在保存…" : item ? "保存修改" : "发布公告"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function AnnouncementsPanel() {
  const queryClient = useQueryClient();
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [editorOpen, setEditorOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<Announcement | null>(null);
  const [deleting, setDeleting] = React.useState<Announcement | null>(null);
  const cursor = cursorStack.at(-1);
  const announcements = useQuery({
    queryKey: ["admin", "announcements", cursor],
    queryFn: () => api.adminAnnouncements(cursor),
  });
  const remove = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => api.deleteAdminAnnouncement(id, reason),
    onSuccess: async () => {
      toast.success("公告已删除");
      setDeleting(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] });
      await queryClient.invalidateQueries({ queryKey: ["home", "announcements"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "删除失败"),
  });

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="社区公告"
        description="发布、修订和撤下首页公告。每次变更都需要原因，删除不会被伪装成普通编辑。"
        actions={
          <Button type="button" size="sm" onClick={() => { setEditing(null); setEditorOpen(true); }}>
            <Plus className="size-4" />发布公告
          </Button>
        }
      />

      {announcements.isLoading ? (
        <LoadingState label="加载公告" />
      ) : announcements.isError ? (
        <ErrorState title="公告加载失败" error={announcements.error} onRetry={() => void announcements.refetch()} />
      ) : (announcements.data?.items ?? []).length === 0 ? (
        <EmptyState title="还没有公告" description="发布后会出现在社区首页公告区。" />
      ) : (
        <div className="space-y-3">
          {announcements.data?.items?.map((item) => (
            <Card key={item.id} className="rounded-xl">
              <CardContent className="flex flex-col gap-4 p-4 md:flex-row md:items-start md:justify-between">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <Megaphone className="size-4 shrink-0 text-primary" aria-hidden="true" />
                    <h3 className="font-medium">{item.title}</h3>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap text-sm leading-6 text-muted-foreground">{item.body || "无正文"}</p>
                  <p className="mt-2 text-xs text-muted-foreground">发布于 {formatUnixTime(item.createdAt)}</p>
                </div>
                <div className="flex shrink-0 gap-2">
                  <Button type="button" variant="outline" size="sm" onClick={() => { setEditing(item); setEditorOpen(true); }}>
                    <Pencil className="size-4" />编辑
                  </Button>
                  <Button type="button" variant="destructive" size="sm" onClick={() => setDeleting(item)}>
                    <Trash2 className="size-4" />删除
                  </Button>
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
      <ReasonDialog
        open={Boolean(deleting)}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={`删除公告“${deleting?.title ?? ""}”`}
        description="公告会立即从公共页面移除，审计事件仍会保留。"
        confirmLabel="确认删除"
        destructive
        isPending={remove.isPending}
        onConfirm={(reason) => deleting && remove.mutate({ id: deleting.id, reason })}
      />
    </div>
  );
}


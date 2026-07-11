import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2, Pencil, Trash2 } from "lucide-react";
import * as React from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";

import { MarkdownEditor } from "@/components/content/markdown-editor";
import { Button } from "@/components/ui/button";
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
import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type { Comment, ContentFormat, ThreadDetailWithPoll } from "@/lib/api/types";

interface VersionedSource {
  body: string;
  contentFormat: ContentFormat;
  contentVersion: number;
}

interface ThreadSource extends VersionedSource {
  title: string;
}

function isVersionConflict(error: unknown) {
  return error instanceof ApiError && error.status === 409 && error.code === "VERSION_CONFLICT";
}

function ConflictNotice({
  onUseLatest,
  onRetry,
  isPending,
}: {
  onUseLatest: () => void;
  onRetry: () => void;
  isPending: boolean;
}) {
  return (
    <div role="alert" className="motion-pop space-y-3 rounded-lg border border-primary/35 bg-primary/10 p-3">
      <div>
        <p className="text-sm font-medium">线上内容已在其他位置更新</p>
        <p className="mt-1 text-xs text-muted-foreground">
          你的输入仍保留。可以载入线上版本，或确认后用当前输入基于最新版重试。
        </p>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button type="button" size="sm" variant="outline" onClick={onUseLatest} disabled={isPending}>
          载入线上版本
        </Button>
        <Button type="button" size="sm" onClick={onRetry} disabled={isPending}>
          保留我的内容并重试
        </Button>
      </div>
    </div>
  );
}

function DeleteConfirmation({
  open,
  onOpenChange,
  noun,
  isPending,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  noun: "帖子" | "回复";
  isPending: boolean;
  onConfirm: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => { if (!isPending) onOpenChange(next); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>删除{noun}</DialogTitle>
          <DialogDescription>
            这会软删除{noun}并从公共页面移除；治理记录仍会保留，管理员可以按社区政策恢复。
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isPending}>
            取消
          </Button>
          <Button type="button" variant="destructive" onClick={onConfirm} disabled={isPending}>
            {isPending ? <Loader2 className="size-4 motion-safe:animate-spin" aria-hidden="true" /> : <Trash2 className="size-4" aria-hidden="true" />}
            确认删除
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function threadSource(thread: ThreadDetailWithPoll): ThreadSource {
  return {
    title: thread.title ?? "",
    body: thread.body ?? "",
    contentFormat: thread.contentFormat ?? "plain_v1",
    contentVersion: thread.contentVersion ?? 1,
  };
}

function commentSource(comment: Comment): VersionedSource {
  return {
    body: comment.body ?? "",
    contentFormat: comment.contentFormat ?? "plain_v1",
    contentVersion: comment.contentVersion ?? 1,
  };
}

export function ThreadAuthorActions({ thread }: { thread: ThreadDetailWithPoll }) {
  const threadId = thread.id ?? "";
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [isEditing, setIsEditing] = React.useState(false);
  const [isDeleting, setIsDeleting] = React.useState(false);
  const [draft, setDraft] = React.useState<ThreadSource>(() => threadSource(thread));
  const [conflict, setConflict] = React.useState<ThreadSource | null>(null);

  const invalidate = React.useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["thread", threadId] }),
      queryClient.invalidateQueries({ queryKey: ["forum", "threads"] }),
      queryClient.invalidateQueries({ queryKey: ["home", "threads"] }),
      queryClient.invalidateQueries({ queryKey: ["profile", thread.authorHandle] }),
    ]);
  }, [queryClient, thread.authorHandle, threadId]);

  const update = useMutation({
    mutationFn: (source: ThreadSource) => api.updateThread(threadId, {
      expectedVersion: source.contentVersion,
      title: source.title,
      body: source.body,
      contentFormat: "markdown_v1",
    }),
    onSuccess: async () => {
      setConflict(null);
      setIsEditing(false);
      toast.success("帖子已更新");
      await invalidate();
    },
    onError: async (error) => {
      if (!isVersionConflict(error)) {
        toast.error(error instanceof Error ? error.message : "帖子更新失败");
        return;
      }
      try {
        const latest = await api.thread(threadId);
        setConflict(threadSource(latest));
      } catch (refreshError) {
        toast.error(refreshError instanceof Error ? refreshError.message : "无法读取线上版本");
      }
    },
  });
  const remove = useMutation({
    mutationFn: () => api.deleteThread(threadId),
    onSuccess: async () => {
      setIsDeleting(false);
      toast.success("帖子已删除");
      await invalidate();
      navigate("/forum");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "帖子删除失败"),
  });

  function openEditor() {
    setDraft(threadSource(thread));
    setConflict(null);
    setIsEditing(true);
  }

  return (
    <>
      {thread.canEdit ? (
        <Button type="button" variant="outline" onClick={openEditor}>
          <Pencil className="size-4" aria-hidden="true" />
          编辑
        </Button>
      ) : null}
      {thread.canDelete ? (
        <Button type="button" variant="ghost" onClick={() => setIsDeleting(true)}>
          <Trash2 className="size-4" aria-hidden="true" />
          删除
        </Button>
      ) : null}

      <Dialog open={isEditing} onOpenChange={(next) => { if (!update.isPending) setIsEditing(next); }}>
        <DialogContent className="max-h-[calc(100dvh-2rem)] max-w-3xl overflow-y-auto">
          <DialogHeader>
            <DialogTitle>编辑帖子</DialogTitle>
            <DialogDescription>保存时会检查内容版本；检测到其他设备修改时不会覆盖你的输入。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            {draft.contentFormat === "plain_v1" ? (
              <p role="note" className="rounded-lg border bg-muted/40 p-3 text-xs text-muted-foreground">
                这是旧版纯文本内容。保存会显式升级为 Markdown，请先在预览标签确认显示结果。
              </p>
            ) : null}
            {conflict ? (
              <ConflictNotice
                isPending={update.isPending}
                onUseLatest={() => { setDraft(conflict); setConflict(null); }}
                onRetry={() => update.mutate({ ...draft, contentVersion: conflict.contentVersion })}
              />
            ) : null}
            <div className="space-y-2">
              <Label htmlFor={`thread-title-${threadId}`}>标题</Label>
              <Input
                id={`thread-title-${threadId}`}
                value={draft.title}
                onChange={(event) => setDraft((current) => ({ ...current, title: event.target.value }))}
                maxLength={200}
              />
            </div>
            <MarkdownEditor
              value={draft.body}
              onChange={(body) => setDraft((current) => ({ ...current, body }))}
              label="帖子正文"
              maxLength={50_000}
              minHeight={260}
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setIsEditing(false)} disabled={update.isPending}>
              取消
            </Button>
            <Button
              type="button"
              onClick={() => update.mutate(draft)}
              disabled={!draft.title.trim() || update.isPending || Boolean(conflict)}
            >
              {update.isPending ? <Loader2 className="size-4 motion-safe:animate-spin" aria-hidden="true" /> : null}
              保存修改
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <DeleteConfirmation
        open={isDeleting}
        onOpenChange={setIsDeleting}
        noun="帖子"
        isPending={remove.isPending}
        onConfirm={() => remove.mutate()}
      />
    </>
  );
}

export function CommentAuthorActions({ comment, threadId }: { comment: Comment; threadId: string }) {
  const commentId = comment.id ?? "";
  const queryClient = useQueryClient();
  const [isEditing, setIsEditing] = React.useState(false);
  const [isDeleting, setIsDeleting] = React.useState(false);
  const [draft, setDraft] = React.useState<VersionedSource>(() => commentSource(comment));
  const [conflict, setConflict] = React.useState<VersionedSource | null>(null);

  const invalidate = React.useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["thread", threadId] }),
      queryClient.invalidateQueries({ queryKey: ["thread-comments", threadId] }),
      queryClient.invalidateQueries({ queryKey: ["forum", "threads"] }),
      queryClient.invalidateQueries({ queryKey: ["home", "threads"] }),
      queryClient.invalidateQueries({ queryKey: ["profile", comment.authorHandle] }),
    ]);
  }, [comment.authorHandle, queryClient, threadId]);

  const update = useMutation({
    mutationFn: (source: VersionedSource) => api.updateComment(commentId, {
      expectedVersion: source.contentVersion,
      body: source.body,
      contentFormat: "markdown_v1",
    }),
    onSuccess: async () => {
      setConflict(null);
      setIsEditing(false);
      toast.success("回复已更新");
      await invalidate();
    },
    onError: async (error) => {
      if (!isVersionConflict(error)) {
        toast.error(error instanceof Error ? error.message : "回复更新失败");
        return;
      }
      try {
        const page = await api.comments(threadId);
        const latest = page.items?.find((item) => item.id === commentId);
        if (!latest) throw new Error("无法在当前楼层中找到线上版本");
        setConflict(commentSource(latest));
      } catch (refreshError) {
        toast.error(refreshError instanceof Error ? refreshError.message : "无法读取线上版本");
      }
    },
  });
  const remove = useMutation({
    mutationFn: () => api.deleteComment(commentId),
    onSuccess: async () => {
      setIsDeleting(false);
      toast.success("回复已删除");
      await invalidate();
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "回复删除失败"),
  });

  function openEditor() {
    setDraft(commentSource(comment));
    setConflict(null);
    setIsEditing(true);
  }

  return (
    <>
      {comment.canEdit ? (
        <Button type="button" size="sm" variant="ghost" onClick={openEditor}>
          <Pencil className="size-4" aria-hidden="true" />
          编辑
        </Button>
      ) : null}
      {comment.canDelete ? (
        <Button type="button" size="sm" variant="ghost" onClick={() => setIsDeleting(true)}>
          <Trash2 className="size-4" aria-hidden="true" />
          删除
        </Button>
      ) : null}

      <Dialog open={isEditing} onOpenChange={(next) => { if (!update.isPending) setIsEditing(next); }}>
        <DialogContent className="max-h-[calc(100dvh-2rem)] max-w-2xl overflow-y-auto">
          <DialogHeader>
            <DialogTitle>编辑回复</DialogTitle>
            <DialogDescription>保存时会检查内容版本；冲突时当前输入会保留在编辑器中。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            {draft.contentFormat === "plain_v1" ? (
              <p role="note" className="rounded-lg border bg-muted/40 p-3 text-xs text-muted-foreground">
                这是旧版纯文本内容。保存会显式升级为 Markdown，请先确认预览。
              </p>
            ) : null}
            {conflict ? (
              <ConflictNotice
                isPending={update.isPending}
                onUseLatest={() => { setDraft(conflict); setConflict(null); }}
                onRetry={() => update.mutate({ ...draft, contentVersion: conflict.contentVersion })}
              />
            ) : null}
            <MarkdownEditor
              value={draft.body}
              onChange={(body) => setDraft((current) => ({ ...current, body }))}
              label="回复正文"
              maxLength={16_000}
              minHeight={200}
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setIsEditing(false)} disabled={update.isPending}>
              取消
            </Button>
            <Button
              type="button"
              onClick={() => update.mutate(draft)}
              disabled={!draft.body.trim() || update.isPending || Boolean(conflict)}
            >
              {update.isPending ? <Loader2 className="size-4 motion-safe:animate-spin" aria-hidden="true" /> : null}
              保存修改
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <DeleteConfirmation
        open={isDeleting}
        onOpenChange={setIsDeleting}
        noun="回复"
        isPending={remove.isPending}
        onConfirm={() => remove.mutate()}
      />
    </>
  );
}

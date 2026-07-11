import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bookmark, Flag, MessageSquare, Send, ThumbsDown, ThumbsUp } from "lucide-react";
import * as React from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { MarkdownContent } from "@/components/content/markdown-content";
import { MarkdownEditor } from "@/components/content/markdown-editor";
import { DraftSyncNotice } from "@/components/forum/draft-sync-notice";
import {
  CommentAuthorActions,
  ThreadAuthorActions,
} from "@/components/forum/author-content-actions";
import {
  CommentModerationControls,
  ThreadModerationControls,
} from "@/components/forum/moderation-controls";
import { useForumDraft } from "@/components/forum/use-forum-draft";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Comment, DraftPayload } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

function CommentCard({ comment, threadId }: { comment: Comment; threadId: string }) {
  const queryClient = useQueryClient();
  const vote = useMutation({
    mutationFn: (value: "up" | "down") =>
      comment.viewerVote === value
        ? api.removePostVote(comment.id ?? "", "comment")
        : api.votePost(comment.id ?? "", value, "comment"),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["thread-comments", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });
  const flag = useMutation({
    mutationFn: () => api.flagPost(comment.id ?? "", "other", undefined, "comment"),
    onSuccess: () => toast.success("已提交举报"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "举报失败"),
  });
  const bookmark = useMutation({
    mutationFn: () =>
      comment.isBookmarked
        ? api.removeBookmark(comment.id ?? "", "comment")
        : api.bookmarkPost(comment.id ?? "", "comment"),
    onSuccess: async () => {
      toast.success(comment.isBookmarked ? "已取消收藏" : "已收藏");
      await queryClient.invalidateQueries({ queryKey: ["thread-comments", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "收藏失败"),
  });
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <span className="font-medium">{comment.authorHandle}</span>
            <span className="text-muted-foreground">{formatUnixTime(comment.createdAt)}</span>
            {comment.isHidden ? <Badge variant="outline">已隐藏</Badge> : null}
            {comment.isDeleted ? <Badge variant="outline">已删除</Badge> : null}
          </div>
          <div className="flex items-center gap-1">
            <Badge variant="secondary">{comment.voteCount ?? 0}</Badge>
            <CommentAuthorActions comment={comment} threadId={threadId} />
            <CommentModerationControls comment={comment} threadId={threadId} />
          </div>
        </div>
        <MarkdownContent
          content={comment.body}
          format={comment.contentFormat}
          className="mt-3 text-sm"
        />
        <div className="mt-3 flex gap-2">
          <Button
            size="sm"
            variant={comment.viewerVote === "up" ? "secondary" : "ghost"}
            onClick={() => vote.mutate("up")}
            disabled={vote.isPending}
          >
            <ThumbsUp className="h-4 w-4" />
            顶
          </Button>
          <Button
            size="sm"
            variant={comment.viewerVote === "down" ? "secondary" : "ghost"}
            onClick={() => vote.mutate("down")}
            disabled={vote.isPending}
          >
            <ThumbsDown className="h-4 w-4" />
            踩
          </Button>
          <Button
            size="sm"
            variant={comment.isBookmarked ? "secondary" : "ghost"}
            onClick={() => bookmark.mutate()}
            disabled={bookmark.isPending}
          >
            <Bookmark className="h-4 w-4" />
            {comment.isBookmarked ? "已收藏" : "收藏"}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => flag.mutate()} disabled={flag.isPending}>
            <Flag className="h-4 w-4" />
            举报
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function PollCard({
  poll,
  threadId,
}: {
  poll: NonNullable<Awaited<ReturnType<typeof api.thread>>["poll"]>;
  threadId: string;
}) {
  const queryClient = useQueryClient();
  const totalVotes = (poll.options ?? []).reduce((sum, option) => sum + (option.voteCount ?? 0), 0);
  const isClosed = poll.closesAt != null && poll.closesAt <= Math.floor(Date.now() / 1000);
  const vote = useMutation({
    mutationFn: (optionId: string) =>
      (poll.myVotes ?? []).includes(optionId)
        ? api.removePollVote(poll.id ?? "", optionId)
        : api.votePoll(poll.id ?? "", optionId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["thread", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "投票失败"),
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>{poll.question ?? "投票"}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-2">
        {(poll.options ?? []).map((option) => {
          const count = option.voteCount ?? 0;
          const percent = totalVotes > 0 ? Math.round((count / totalVotes) * 100) : 0;
          const hasVoted = (poll.myVotes ?? []).includes(option.id ?? "");
          return (
            <button
              key={option.id}
              onClick={() => option.id && vote.mutate(option.id)}
              disabled={!option.id || vote.isPending || isClosed}
              className={`w-full rounded-md border p-3 text-left transition-colors hover:bg-accent disabled:opacity-70 ${hasVoted ? "border-primary bg-primary/5" : ""}`}
            >
              <div className="flex items-center justify-between gap-3">
                <span className="font-medium">{option.label}</span>
                <span className="text-sm text-muted-foreground">{count} 票 · {percent}%</span>
              </div>
              <div className="mt-2 h-2 overflow-hidden rounded-full bg-muted">
                <div className="h-full bg-primary" style={{ width: `${percent}%` }} />
              </div>
              {hasVoted ? <p className="mt-1 text-xs text-primary">已选择</p> : null}
            </button>
          );
        })}
        <p className="text-xs text-muted-foreground">
          共 {totalVotes} 票{poll.multiSelect ? " · 可多选" : ""}
          {isClosed ? " · 已截止" : ""}
        </p>
      </CardContent>
    </Card>
  );
}

function CommentForm({ threadId }: { threadId: string }) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [body, setBody] = React.useState("");
  const payload = React.useMemo<Extract<DraftPayload, { kind: "comment" }>>(() => ({
    kind: "comment",
    threadId,
    body,
    contentFormat: "markdown_v1",
    parentId: null,
  }), [body, threadId]);
  const restoreDraft = React.useCallback((draftPayload: Extract<DraftPayload, { kind: "comment" }>) => {
    if (draftPayload.threadId === threadId) setBody(draftPayload.body);
  }, [threadId]);
  const draft = useForumDraft({
    draftKey: `comment:${threadId}`,
    enabled: isAuthenticated && Boolean(threadId),
    isEmpty: !body,
    payload,
    onRestore: restoreDraft,
  });
  const mutation = useMutation({
    mutationFn: () => api.addComment(threadId, body),
    onSuccess: async () => {
      toast.success("回复已发布");
      await draft.clearDraft().catch(() => toast.warning("回复已发布，但云端草稿清理失败"));
      setBody("");
      await queryClient.invalidateQueries({ queryKey: ["thread-comments", threadId] });
      await queryClient.invalidateQueries({ queryKey: ["thread", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "回复失败"),
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后回复" description="登录后可以参与讨论、投票、收藏和举报。" />;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>回复</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <DraftSyncNotice
          status={draft.status}
          savedAt={draft.savedAt}
          onRestoreRemote={draft.restoreRemote}
          onKeepLocal={draft.keepLocal}
          onRetry={draft.retry}
        />
        <MarkdownEditor
          value={body}
          onChange={setBody}
          label="回复正文"
          maxLength={16_000}
          minHeight={140}
          placeholder="写下你的回复"
        />
        <Button onClick={() => mutation.mutate()} disabled={!body.trim() || mutation.isPending}>
          <Send className="h-4 w-4" />
          发布回复
        </Button>
      </CardContent>
    </Card>
  );
}

export function ThreadDetailPage() {
  const { id } = useParams();
  const threadId = id ?? "";
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuth();
  const thread = useQuery({
    queryKey: ["thread", threadId],
    queryFn: () => api.thread(threadId),
    enabled: Boolean(threadId),
  });
  const boards = useQuery({ queryKey: ["forum", "boards"], queryFn: api.boards });
  const comments = useQuery({
    queryKey: ["thread-comments", threadId],
    queryFn: () => api.comments(threadId),
    enabled: Boolean(threadId),
  });
  const vote = useMutation({
    mutationFn: (value: "up" | "down") =>
      thread.data?.viewerVote === value
        ? api.removePostVote(threadId, "thread")
        : api.votePost(threadId, value, "thread"),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["thread", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "投票失败"),
  });
  const bookmark = useMutation({
    mutationFn: () =>
      thread.data?.isBookmarked
        ? api.removeBookmark(threadId, "thread")
        : api.bookmarkPost(threadId, "thread"),
    onSuccess: async () => {
      toast.success(thread.data?.isBookmarked ? "已取消收藏" : "已收藏");
      await queryClient.invalidateQueries({ queryKey: ["thread", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "收藏失败"),
  });
  const flag = useMutation({
    mutationFn: () => api.flagPost(threadId, "other", undefined, "thread"),
    onSuccess: () => toast.success("已提交举报"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "举报失败"),
  });
  const subscribe = useMutation({
    mutationFn: (level: string) =>
      level === "none"
        ? api.deleteSubscription({ targetType: "thread", targetId: threadId })
        : api.setSubscription({ targetType: "thread", targetId: threadId, level }),
    onSuccess: async () => {
      toast.success("订阅已更新");
      await queryClient.invalidateQueries({ queryKey: ["thread", threadId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "订阅失败"),
  });

  React.useEffect(() => {
    if (!isAuthenticated || !threadId || !thread.data?.id) {
      return;
    }
    void api
      .reportThreadRead(threadId, null)
      .then(() => queryClient.invalidateQueries({ queryKey: ["forum", "threads"] }))
      .catch(() => undefined);
  }, [isAuthenticated, queryClient, thread.data?.id, threadId]);

  if (thread.isLoading) {
    return <LoadingState label="加载帖子" />;
  }
  if (thread.isError || !thread.data) {
    return <ErrorState error={thread.error} onRetry={() => void thread.refetch()} />;
  }

  const item = thread.data;
  const board = boards.data?.find((boardItem) => boardItem.id === item.boardId);
  const replyUnavailableReason = item.deletedAt
    ? "帖子已被软删除，恢复后才能继续回复。"
    : item.hiddenAt
      ? "帖子正在治理隐藏状态，恢复公开后才能继续回复。"
      : item.archivedAt
        ? "帖子已归档，不再接受新回复。"
        : item.closedAt
          ? "帖子已由版主关闭，不再接受新回复。"
          : null;

  return (
    <div className="space-y-5">
      <PageHeader
        eyebrow={board?.name ?? "Forum"}
        title={item.title ?? "帖子详情"}
        description={`${item.authorHandle} · ${formatUnixTime(item.createdAt)}`}
        actions={
          <>
            <Select value={item.mySubscriptionLevel ?? "none"} onValueChange={(value) => subscribe.mutate(value)}>
              <SelectTrigger className="w-32" aria-label="主题订阅级别">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">不订阅</SelectItem>
                <SelectItem value="watching">订阅通知</SelectItem>
                <SelectItem value="tracking">跟踪</SelectItem>
                <SelectItem value="muted">静音</SelectItem>
              </SelectContent>
            </Select>
            <Button
              variant={item.isBookmarked ? "secondary" : "outline"}
              onClick={() => bookmark.mutate()}
              disabled={bookmark.isPending}
            >
              <Bookmark className="h-4 w-4" />
              {item.isBookmarked ? "已收藏" : "收藏"}
            </Button>
            <ThreadAuthorActions thread={item} />
            <ThreadModerationControls thread={item} boards={boards.data ?? []} />
          </>
        }
      />

      <Card>
        <CardContent className="p-5">
          <div className="mb-3 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
            <Badge variant="secondary">{item.replyCount ?? 0} 回复</Badge>
            <Badge variant="secondary">{item.voteCount ?? 0} 票</Badge>
            {item.pinnedAt ? <Badge>置顶</Badge> : null}
            {item.closedAt ? <Badge variant="outline">已关闭</Badge> : null}
            {item.archivedAt ? <Badge variant="outline">已归档</Badge> : null}
            {item.hiddenAt ? <Badge variant="outline">已隐藏</Badge> : null}
            {item.deletedAt ? <Badge variant="destructive">已删除</Badge> : null}
            {(item.tags ?? []).map((tag) => <Badge key={tag} variant="outline">#{tag}</Badge>)}
          </div>
          {item.body ? (
            <MarkdownContent content={item.body} format={item.contentFormat} className="text-sm" />
          ) : (
            <p className="text-sm text-muted-foreground">这条帖子没有正文。</p>
          )}
          <div className="mt-5 flex flex-wrap gap-2">
            <Button
              variant={item.viewerVote === "up" ? "default" : "secondary"}
              onClick={() => vote.mutate("up")}
              disabled={vote.isPending}
            >
              <ThumbsUp className="h-4 w-4" />
              顶
            </Button>
            <Button
              variant={item.viewerVote === "down" ? "default" : "secondary"}
              onClick={() => vote.mutate("down")}
              disabled={vote.isPending}
            >
              <ThumbsDown className="h-4 w-4" />
              踩
            </Button>
            <Button variant="ghost" onClick={() => flag.mutate()} disabled={flag.isPending}>
              <Flag className="h-4 w-4" />
              举报
            </Button>
          </div>
        </CardContent>
      </Card>

      {item.poll ? <PollCard poll={item.poll} threadId={threadId} /> : null}

      {replyUnavailableReason ? (
        <EmptyState title="当前帖子不可回复" description={replyUnavailableReason} />
      ) : (
        <CommentForm threadId={threadId} />
      )}

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <MessageSquare className="h-4 w-4 text-primary" />
          <h2 className="font-semibold">楼层</h2>
        </div>
        {comments.isLoading ? (
          <LoadingState />
        ) : comments.isError ? (
          <ErrorState error={comments.error} onRetry={() => void comments.refetch()} />
        ) : (comments.data?.items ?? []).length === 0 ? (
          <EmptyState title="暂无回复" description="来补充第一条回复。" />
        ) : (
          (comments.data?.items ?? []).map((comment) => (
            <CommentCard key={comment.id} comment={comment} threadId={threadId} />
          ))
        )}
      </section>

      <Button asChild variant="outline">
        <Link to="/forum">返回论坛</Link>
      </Button>
    </div>
  );
}

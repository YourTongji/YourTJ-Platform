import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Eye, EyeOff, LockKeyhole, MessageSquareWarning, Search, ShieldAlert, Undo2 } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { CommentModerationMenu, ThreadModerationMenu } from "@/components/forum/moderation-controls";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api } from "@/lib/api/endpoints";
import type { AdminForumFlag, DmReport, Review, ReviewReport } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

function ReviewQueue() {
  const queryClient = useQueryClient();
  const [decision, setDecision] = React.useState<{ review: Review; action: "toggle" | "delete" } | null>(null);
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const cursor = cursorStack.at(-1);
  const reviews = useQuery({
    queryKey: ["admin", "reviews", "all", cursor],
    queryFn: () => api.adminReviews("all", cursor),
  });
  const update = useMutation({
    mutationFn: async ({ id, action, reason }: { id: string; action: "toggle" | "delete"; reason: string }) => {
      if (action === "toggle") {
        await api.toggleReview(id, reason);
      } else {
        await api.deleteAdminReview(id, reason);
      }
    },
    onSuccess: async () => {
      toast.success("点评治理操作已完成");
      setDecision(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "reviews"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "操作失败"),
  });

  if (reviews.isLoading) return <LoadingState label="加载点评审核队列" />;
  if (reviews.isError) return <ErrorState error={reviews.error} onRetry={() => void reviews.refetch()} />;
  if ((reviews.data?.items ?? []).length === 0 && cursorStack.length === 1) {
    return <EmptyState title="没有点评" />;
  }

  return (
    <div className="space-y-3">
      {reviews.data?.items?.map((review) => {
        const willShow = review.status === "hidden" || review.status === "pending";
        return (
          <Card key={review.id}>
            <CardContent className="flex flex-col gap-3 p-4 md:flex-row md:items-center md:justify-between">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <AdminStatusBadge value={review.status} />
                  <span className="font-medium">{review.authorHandle}</span>
                  <span className="text-xs text-muted-foreground">{review.rating} 星 · {formatUnixTime(review.createdAt)}</span>
                </div>
                <p className="mt-2 line-clamp-3 text-sm leading-6">{review.comment ?? "无正文"}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button type="button" variant="outline" size="sm" onClick={() => setDecision({ review, action: "toggle" })}>
                  {willShow ? <Eye className="size-4" /> : <EyeOff className="size-4" />}
                  {review.status === "pending" ? "批准公开" : willShow ? "恢复公开" : "隐藏点评"}
                </Button>
                <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ review, action: "delete" })}>
                  移除
                </Button>
              </div>
            </CardContent>
          </Card>
        );
      })}
      {(reviews.data?.items ?? []).length === 0 ? <EmptyState title="本页没有点评" /> : null}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(reviews.data?.hasMore && reviews.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => reviews.data?.nextCursor && setCursorStack((items) => [...items, reviews.data?.nextCursor ?? null])}
      />
      <ReasonDialog
        open={Boolean(decision)}
        onOpenChange={(open) => !open && setDecision(null)}
        title={decision?.action === "delete"
          ? "移除点评"
          : decision?.review.status === "pending"
            ? "批准点评公开"
            : decision?.review.status === "hidden"
              ? "恢复点评公开"
              : "隐藏点评"}
        description={decision?.action === "delete"
          ? "移除采用可审计的软删除，不会直接清除历史记录。"
          : "显隐变更会立即影响公共课程页面，并记录操作人、角色和原因。"}
        confirmLabel={decision?.action === "delete" ? "确认移除" : "确认更新"}
        destructive={decision?.action === "delete" || decision?.review.status === "visible"}
        isPending={update.isPending}
        onConfirm={(reason) => decision?.review.id && update.mutate({ id: decision.review.id, action: decision.action, reason })}
      />
    </div>
  );
}

function ReviewReportsQueue() {
  const queryClient = useQueryClient();
  const [decision, setDecision] = React.useState<{ report: ReviewReport; action: "uphold" | "reject" | "ignore" } | null>(null);
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const cursor = cursorStack.at(-1);
  const reports = useQuery({
    queryKey: ["admin", "review-reports", "open", cursor],
    queryFn: () => api.adminReports("open", cursor),
  });
  const resolve = useMutation({
    mutationFn: ({ id, action, reason }: { id: string; action: "uphold" | "reject" | "ignore"; reason: string }) =>
      api.resolveReport(id, action, reason),
    onSuccess: async () => {
      toast.success("点评举报已处理");
      setDecision(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "review-reports"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "处理失败"),
  });

  if (reports.isLoading) return <LoadingState label="加载点评举报" />;
  if (reports.isError) return <ErrorState error={reports.error} onRetry={() => void reports.refetch()} />;
  if ((reports.data?.items ?? []).length === 0 && cursorStack.length === 1) {
    return <EmptyState title="没有开放的点评举报" />;
  }

  return (
    <div className="space-y-3">
      {reports.data?.items?.map((report) => (
        <Card key={report.id}>
          <CardContent className="flex flex-col gap-3 p-4 md:flex-row md:items-start md:justify-between">
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <AdminStatusBadge value={report.status} />
                {report.reviewStatus ? <AdminStatusBadge value={report.reviewStatus} /> : null}
                <span className="text-xs text-muted-foreground">点评 {report.reviewId} · {formatUnixTime(report.createdAt)}</span>
              </div>
              <div className="mt-3 rounded-lg border bg-muted/40 p-3">
                <p className="text-xs text-muted-foreground">
                  {report.courseId ? `课程 ${report.courseId} · ` : ""}
                  作者 {report.reviewAuthorHandle ?? "未知"}
                  {typeof report.reviewRating === "number" ? ` · ${report.reviewRating} 星` : ""}
                </p>
                <p className="mt-2 whitespace-pre-wrap text-sm leading-6">
                  {report.reviewExcerpt || "没有可展示的点评摘要，请暂缓裁决并核对原始记录。"}
                </p>
              </div>
              <p className="mt-2 text-sm"><span className="font-medium">举报理由：</span>{report.reason}</p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ report, action: "uphold" })}>举报成立</Button>
              <Button type="button" variant="outline" size="sm" onClick={() => setDecision({ report, action: "reject" })}>驳回举报</Button>
              <Button type="button" variant="ghost" size="sm" onClick={() => setDecision({ report, action: "ignore" })}>忽略</Button>
            </div>
          </CardContent>
        </Card>
      ))}
      {(reports.data?.items ?? []).length === 0 ? <EmptyState title="本页没有开放的点评举报" /> : null}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(reports.data?.hasMore && reports.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => reports.data?.nextCursor && setCursorStack((items) => [...items, reports.data?.nextCursor ?? null])}
      />
      <ReasonDialog
        open={Boolean(decision)}
        onOpenChange={(open) => !open && setDecision(null)}
        title={decision?.action === "uphold" ? "确认点评举报成立" : decision?.action === "reject" ? "确认驳回点评举报" : "确认忽略点评举报"}
        description="处理结果会写入明确状态，并与操作备注一起保留在治理审计中。"
        confirmLabel="提交决定"
        destructive={decision?.action === "uphold"}
        isPending={resolve.isPending}
        onConfirm={(reason) => decision?.report.id && resolve.mutate({ id: decision.report.id, action: decision.action, reason })}
      />
    </div>
  );
}

function ForumFlagsQueue() {
  const queryClient = useQueryClient();
  const [decision, setDecision] = React.useState<{ flag: AdminForumFlag; action: "uphold" | "reject" | "ignore" } | null>(null);
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const cursor = cursorStack.at(-1);
  const flags = useQuery({
    queryKey: ["admin", "forum-flags", "open", cursor],
    queryFn: () => api.adminForumFlags("open", cursor),
  });
  const resolve = useMutation({
    mutationFn: ({ id, action, reason }: { id: string; action: "uphold" | "reject" | "ignore"; reason: string }) =>
      api.resolveAdminForumFlag(id, action, reason),
    onSuccess: async () => {
      toast.success("论坛举报决定已提交");
      setDecision(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "forum-flags"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "处理失败"),
  });

  if (flags.isLoading) return <LoadingState label="加载论坛举报" />;
  if (flags.isError) return <ErrorState error={flags.error} onRetry={() => void flags.refetch()} />;
  if ((flags.data?.items ?? []).length === 0 && cursorStack.length === 1) {
    return <EmptyState title="没有开放的论坛举报" />;
  }

  return (
    <div className="space-y-3">
      {flags.data?.items?.map((flag) => (
        <Card key={flag.id}>
          <CardContent className="flex flex-col gap-3 p-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <AdminStatusBadge value={flag.status} />
                <Badge variant="outline">{flag.targetType === "thread" ? "帖子" : "回复"} #{flag.targetId}</Badge>
                <span className="text-xs text-muted-foreground">权重 {flag.weight} · {formatUnixTime(flag.createdAt)}</span>
              </div>
              <div className="mt-3 rounded-lg border bg-muted/40 p-3">
                <p className="text-xs text-muted-foreground">
                  作者 {flag.authorHandle ?? "未知"}
                  {flag.targetTitle ? ` · ${flag.targetTitle}` : ""}
                </p>
                <p className="mt-2 whitespace-pre-wrap text-sm leading-6">
                  {flag.contentExcerpt || "没有可展示的内容摘要，请暂缓裁决并使用“内容恢复”核对目标。"}
                </p>
              </div>
              <p className="mt-2 text-sm"><span className="font-medium">举报理由：</span>{flag.reason}</p>
              {flag.note ? <p className="mt-1 text-xs text-muted-foreground">举报补充：{flag.note}</p> : null}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ flag, action: "uphold" })}>举报成立</Button>
              <Button type="button" variant="outline" size="sm" onClick={() => setDecision({ flag, action: "reject" })}>驳回举报</Button>
              <Button type="button" variant="ghost" size="sm" onClick={() => setDecision({ flag, action: "ignore" })}>忽略</Button>
            </div>
          </CardContent>
        </Card>
      ))}
      {(flags.data?.items ?? []).length === 0 ? <EmptyState title="本页没有开放的论坛举报" /> : null}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(flags.data?.hasMore && flags.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => flags.data?.nextCursor && setCursorStack((items) => [...items, flags.data?.nextCursor ?? null])}
      />
      <ReasonDialog
        open={Boolean(decision)}
        onOpenChange={(open) => !open && setDecision(null)}
        title={decision?.action === "uphold" ? "确认举报成立" : decision?.action === "reject" ? "确认驳回举报" : "确认忽略举报"}
        description="举报成立会移除对应内容；驳回或忽略只会解除由这组举报触发的自动隐藏，不会覆盖人工隐藏。请先完成证据核对。"
        confirmLabel="提交决定"
        destructive={decision?.action === "uphold"}
        isPending={resolve.isPending}
        onConfirm={(reason) => decision && resolve.mutate({ id: decision.flag.id, action: decision.action, reason })}
      />
    </div>
  );
}

function DmReportsQueue() {
  const queryClient = useQueryClient();
  const [decision, setDecision] = React.useState<{ report: DmReport; action: "uphold" | "reject" } | null>(null);
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const cursor = cursorStack.at(-1);
  const reports = useQuery({
    queryKey: ["admin", "dm-reports", "open", cursor],
    queryFn: () => api.adminDmReports("open", cursor),
  });
  const resolve = useMutation({
    mutationFn: ({ id, action, reason }: { id: string; action: "uphold" | "reject"; reason: string }) =>
      api.resolveAdminDmReport(id, action, reason),
    onSuccess: async () => {
      toast.success("私信举报决定已提交");
      setDecision(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "dm-reports"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "处理失败"),
  });

  if (reports.isLoading) return <LoadingState label="加载私信举报" />;
  if (reports.isError) return <ErrorState error={reports.error} onRetry={() => void reports.refetch()} />;
  if ((reports.data?.items ?? []).length === 0 && cursorStack.length === 1) {
    return <EmptyState title="没有开放的私信举报" />;
  }

  return (
    <div className="space-y-3">
      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="flex gap-3 p-4 text-sm leading-6">
          <LockKeyhole className="mt-1 size-4 shrink-0 text-primary" aria-hidden="true" />
          这里只显示用户主动举报的消息摘要；后台不能浏览未举报会话，处理时也不应复制无关私信内容。
        </CardContent>
      </Card>
      {reports.data?.items?.map((report) => (
        <Card key={report.id}>
          <CardContent className="space-y-3 p-4">
            <div className="flex flex-wrap items-center gap-2">
              <AdminStatusBadge value={report.status} />
              <Badge variant="outline">{report.reason}</Badge>
              <span className="text-xs text-muted-foreground">{formatUnixTime(report.createdAt)}</span>
            </div>
            <div className="rounded-lg border bg-muted/40 p-3">
              <p className="text-xs text-muted-foreground">发送者 {report.senderHandle ?? report.senderId} · 举报人 {report.reporterHandle ?? report.reporterId}</p>
              <p className="mt-2 whitespace-pre-wrap text-sm leading-6">{report.messageExcerpt || "没有可展示的消息摘要"}</p>
            </div>
            {report.note ? <p className="text-xs text-muted-foreground">举报补充：{report.note}</p> : null}
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="destructive" size="sm" onClick={() => setDecision({ report, action: "uphold" })}>举报成立</Button>
              <Button type="button" variant="outline" size="sm" onClick={() => setDecision({ report, action: "reject" })}>驳回举报</Button>
            </div>
          </CardContent>
        </Card>
      ))}
      {(reports.data?.items ?? []).length === 0 ? <EmptyState title="本页没有开放的私信举报" /> : null}
      <PaginationControls
        hasPrevious={cursorStack.length > 1}
        hasMore={Boolean(reports.data?.hasMore && reports.data.nextCursor)}
        onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
        onNext={() => reports.data?.nextCursor && setCursorStack((items) => [...items, reports.data?.nextCursor ?? null])}
      />
      <ReasonDialog
        open={Boolean(decision)}
        onOpenChange={(open) => !open && setDecision(null)}
        title={decision?.action === "uphold" ? "确认私信举报成立" : "确认驳回私信举报"}
        description="决定只针对这份举报证据，不授权查看会话中的其他消息。"
        confirmLabel="提交决定"
        destructive={decision?.action === "uphold"}
        isPending={resolve.isPending}
        onConfirm={(reason) => decision && resolve.mutate({ id: decision.report.id, action: decision.action, reason })}
      />
    </div>
  );
}

type RecoveryTargetType = "thread" | "comment";

function ContentRecovery() {
  const [targetType, setTargetType] = React.useState<RecoveryTargetType>("thread");
  const [targetId, setTargetId] = React.useState("");
  const [lookup, setLookup] = React.useState<{ type: RecoveryTargetType; id: string } | null>(null);
  const normalizedId = targetId.trim();
  const isValidId = /^[1-9]\d*$/.test(normalizedId);
  const thread = useQuery({
    queryKey: ["admin", "forum", "thread", lookup?.type === "thread" ? lookup.id : ""],
    queryFn: () => api.adminForumThread(lookup?.id ?? ""),
    enabled: lookup?.type === "thread",
    retry: false,
  });
  const comment = useQuery({
    queryKey: ["admin", "forum", "comment", lookup?.type === "comment" ? lookup.id : ""],
    queryFn: () => api.adminForumComment(lookup?.id ?? ""),
    enabled: lookup?.type === "comment",
    retry: false,
  });
  const boards = useQuery({
    queryKey: ["forum", "boards"],
    queryFn: api.boards,
    enabled: lookup?.type === "thread",
  });

  function submitLookup(event: React.FormEvent) {
    event.preventDefault();
    if (!isValidId) return;
    setLookup({ type: targetType, id: normalizedId });
  }

  const activeQuery = lookup?.type === "comment" ? comment : thread;

  return (
    <div className="space-y-4">
      <Card className="border-primary/30 bg-primary/5">
        <CardContent className="p-4 text-sm leading-6 text-muted-foreground">
          按内容类型和数字 ID 读取工作人员专用证据视图。该入口可以看到已隐藏或软删除内容，并复用同一套有原因、可审计的恢复操作。
        </CardContent>
      </Card>
      <Card>
        <CardContent className="p-4">
          <form className="grid gap-3 sm:grid-cols-[10rem_minmax(0,1fr)_auto] sm:items-end" onSubmit={submitLookup}>
            <div className="space-y-2">
              <Label htmlFor="recovery-target-type">内容类型</Label>
              <Select value={targetType} onValueChange={(value) => setTargetType(value as RecoveryTargetType)}>
                <SelectTrigger id="recovery-target-type"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="thread">帖子</SelectItem>
                  <SelectItem value="comment">回复</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="recovery-target-id">数字 ID</Label>
              <Input
                id="recovery-target-id"
                value={targetId}
                onChange={(event) => setTargetId(event.target.value)}
                inputMode="numeric"
                pattern="[1-9][0-9]*"
                placeholder="输入举报记录或审计事件中的目标 ID"
                aria-invalid={Boolean(normalizedId) && !isValidId}
              />
            </div>
            <Button type="submit" variant="outline" disabled={!isValidId || activeQuery.isFetching}>
              <Search className="size-4" />查询证据
            </Button>
          </form>
        </CardContent>
      </Card>

      {!lookup ? (
        <EmptyState title="尚未查询内容" description="可从论坛举报或治理审计中复制目标 ID。" />
      ) : activeQuery.isLoading ? (
        <LoadingState label="加载内容证据" />
      ) : activeQuery.isError ? (
        <ErrorState
          title={`找不到${lookup.type === "thread" ? "帖子" : "回复"} #${lookup.id}`}
          error={activeQuery.error}
          onRetry={() => void activeQuery.refetch()}
        />
      ) : lookup.type === "thread" && thread.data ? (
        <Card>
          <CardContent className="space-y-4 p-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">帖子 #{thread.data.id}</Badge>
                  {thread.data.deletedAt ? <Badge variant="destructive">已删除</Badge> : null}
                  {thread.data.hiddenAt ? <Badge variant="outline">已隐藏</Badge> : null}
                  {thread.data.archivedAt ? <Badge variant="outline">已归档</Badge> : null}
                  {thread.data.closedAt ? <Badge variant="outline">已关闭</Badge> : null}
                </div>
                <h3 className="mt-3 font-semibold">{thread.data.title || "未命名帖子"}</h3>
                <p className="mt-1 text-xs text-muted-foreground">
                  作者 {thread.data.authorHandle || "未知"} · 发布于 {formatUnixTime(thread.data.createdAt)}
                </p>
              </div>
              <ThreadModerationMenu thread={thread.data} boards={boards.data ?? []} />
            </div>
            <div className="rounded-lg border bg-muted/30 p-3">
              <p className="whitespace-pre-wrap text-sm leading-6">{thread.data.body || "该帖子没有正文。"}</p>
            </div>
          </CardContent>
        </Card>
      ) : lookup.type === "comment" && comment.data ? (
        <Card>
          <CardContent className="space-y-4 p-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">回复 #{comment.data.id}</Badge>
                  {comment.data.isDeleted ? <Badge variant="destructive">已删除</Badge> : null}
                  {comment.data.isHidden ? <Badge variant="outline">已隐藏</Badge> : null}
                </div>
                <p className="mt-2 text-xs text-muted-foreground">
                  作者 {comment.data.authorHandle || "未知"} · 帖子 #{comment.data.threadId || "未知"} · {formatUnixTime(comment.data.createdAt)}
                </p>
              </div>
              <CommentModerationMenu comment={comment.data} threadId={comment.data.threadId ?? ""} />
            </div>
            <div className="rounded-lg border bg-muted/30 p-3">
              <p className="whitespace-pre-wrap text-sm leading-6">{comment.data.body || "该回复没有正文。"}</p>
            </div>
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}

export function ModerationPanel() {
  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="统一审核"
        description="按领域隔离证据，并使用一致的成立、驳回和忽略流程。私信区只包含用户主动举报的最小证据。"
      />
      <Tabs defaultValue="forum">
        <TabsList className="scrollbar-none h-auto max-w-full justify-start overflow-x-auto">
          <TabsTrigger value="forum"><ShieldAlert className="mr-1 size-4" />论坛举报</TabsTrigger>
          <TabsTrigger value="review-reports">点评举报</TabsTrigger>
          <TabsTrigger value="reviews">点评状态</TabsTrigger>
          <TabsTrigger value="recovery"><Undo2 className="mr-1 size-4" />内容恢复</TabsTrigger>
          <TabsTrigger value="dm"><MessageSquareWarning className="mr-1 size-4" />私信举报</TabsTrigger>
        </TabsList>
        <TabsContent value="forum" className="pt-2"><ForumFlagsQueue /></TabsContent>
        <TabsContent value="review-reports" className="pt-2"><ReviewReportsQueue /></TabsContent>
        <TabsContent value="reviews" className="pt-2"><ReviewQueue /></TabsContent>
        <TabsContent value="recovery" className="pt-2"><ContentRecovery /></TabsContent>
        <TabsContent value="dm" className="pt-2"><DmReportsQueue /></TabsContent>
      </Tabs>
    </div>
  );
}

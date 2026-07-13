import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Flame, Loader2, Plus, Send, ThumbsUp } from "lucide-react";
import * as React from "react";
import { Link, useSearchParams } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import {
  ForumDeliveryImage,
  useForumDeliveryRefresh,
} from "@/components/content/forum-delivery-image";
import { DraftSyncNotice } from "@/components/forum/draft-sync-notice";
import { useForumDraft } from "@/components/forum/use-forum-draft";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Board, DraftPayload, ThreadFeed } from "@/lib/api/types";
import { formatNumber, formatUnixTime } from "@/lib/format";

const MarkdownEditor = React.lazy(() =>
  import("@/components/content/markdown-editor").then((module) => ({
    default: module.MarkdownEditor,
  })),
);

type ForumFeed = "hot" | "new" | "subscriptions" | "following" | "unread";
const forumFeedOptions: ForumFeed[] = ["hot", "new", "subscriptions", "following", "unread"];
const authenticatedFeeds: ForumFeed[] = ["subscriptions", "following", "unread"];

function ThreadCard({
  thread,
  boards,
  onAttachmentDeliveryRefresh,
}: {
  thread: ThreadFeed;
  boards: Board[];
  onAttachmentDeliveryRefresh: () => void;
}) {
  const board = boards.find((item) => item.id === thread.boardId);
  return (
    <Link to={`/forum/threads/${thread.id}`} className="block">
      <Card className="transition-shadow hover:shadow-md">
        <CardContent className="p-5">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">
                  {thread.authorDisplayName ?? `@${thread.authorHandle}`}
                </span>
                {thread.authorDisplayName ? <span>@{thread.authorHandle}</span> : null}
                <span>{formatUnixTime(thread.lastActivityAt ?? thread.createdAt)}</span>
                {board ? <Badge variant="outline">{board.name}</Badge> : null}
                {thread.unreadCount ? <Badge>{thread.unreadCount} 未读</Badge> : null}
              </div>
              <h2 className="line-clamp-2 text-lg font-semibold">{thread.title}</h2>
              {thread.bodyExcerpt ? (
                <p className="mt-2 line-clamp-2 text-sm leading-6 text-muted-foreground">
                  {thread.bodyExcerpt}
                </p>
              ) : null}
              {thread.attachments?.[0] ? (
                <ForumDeliveryImage
                  attachment={thread.attachments[0]}
                  onDeliveryRefresh={onAttachmentDeliveryRefresh}
                  loading="lazy"
                  decoding="async"
                  className="mt-3 max-h-72 w-full rounded-xl border object-cover"
                />
              ) : null}
              <div className="mt-3 flex flex-wrap gap-1.5">
                {(thread.tags ?? []).map((tag) => (
                  <Badge key={tag} variant="secondary">#{tag}</Badge>
                ))}
              </div>
            </div>
            <div className="hidden shrink-0 grid-cols-2 gap-3 text-center sm:grid">
              <div>
                <p className="text-lg font-semibold">{formatNumber(thread.voteCount)}</p>
                <p className="text-xs text-muted-foreground">热度</p>
              </div>
              <div>
                <p className="text-lg font-semibold">{formatNumber(thread.replyCount)}</p>
                <p className="text-xs text-muted-foreground">回复</p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}

function CreateThreadDialog({ boards }: { boards: Board[] }) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [boardId, setBoardId] = React.useState("");
  const [title, setTitle] = React.useState("");
  const [body, setBody] = React.useState("");
  const [tags, setTags] = React.useState("");
  const [pollQuestion, setPollQuestion] = React.useState("");
  const [pollOptions, setPollOptions] = React.useState("");
  const [attachmentAssetIds, setAttachmentAssetIds] = React.useState<string[]>([]);
  const [attachmentsReady, setAttachmentsReady] = React.useState(true);
  const selectedBoard = boards.find((board) => board.id === boardId);
  const draftPayload = React.useMemo<Extract<DraftPayload, { kind: "thread" }>>(() => ({
    kind: "thread",
    boardId: boardId || null,
    title,
    body,
    contentFormat: "markdown_v1",
    tags: tags
      .split(/[,\s，、]+/)
      .map((tag) => tag.trim())
      .filter(Boolean)
      .slice(0, 3),
    pollQuestion,
    pollOptions: pollOptions.split(/\n+/).map((option) => option.trim()).filter(Boolean).slice(0, 20),
    attachmentAssetIds,
  }), [attachmentAssetIds, boardId, body, pollOptions, pollQuestion, tags, title]);
  const restoreDraft = React.useCallback((payload: Extract<DraftPayload, { kind: "thread" }>) => {
    setBoardId(payload.boardId ?? "");
    setTitle(payload.title);
    setBody(payload.body);
    setTags(payload.tags.join(" "));
    setPollQuestion(payload.pollQuestion);
    setPollOptions(payload.pollOptions.join("\n"));
    setAttachmentAssetIds(payload.attachmentAssetIds);
  }, []);
  const draft = useForumDraft({
    draftKey: "thread:new",
    enabled: isAuthenticated && open,
    isEmpty: !boardId && !title && !body && !tags && !pollQuestion && !pollOptions && attachmentAssetIds.length === 0,
    payload: draftPayload,
    onRestore: restoreDraft,
  });
  const mutation = useMutation({
    mutationFn: () =>
      api.createThread({
        boardId,
        title,
        body: body || undefined,
        contentFormat: "markdown_v1",
        attachmentAssetIds,
        tags: tags
          .split(/[,\s，、]+/)
          .map((tag) => tag.trim())
          .filter(Boolean)
          .slice(0, 3),
        poll:
          pollQuestion.trim() && pollOptions.split(/\n+/).filter(Boolean).length >= 2
            ? {
                question: pollQuestion.trim(),
                options: pollOptions
                  .split(/\n+/)
                  .map((option) => option.trim())
                  .filter(Boolean)
                  .slice(0, 20),
              }
            : undefined,
      }),
    onSuccess: async (thread) => {
      toast.success("帖子已发布");
      await draft.clearDraft().catch(() => toast.warning("帖子已发布，但云端草稿清理失败"));
      setOpen(false);
      setBoardId("");
      setTitle("");
      setBody("");
      setTags("");
      setPollQuestion("");
      setPollOptions("");
      setAttachmentAssetIds([]);
      await queryClient.invalidateQueries({ queryKey: ["forum", "threads"] });
      if (thread.id) {
        window.location.href = `/forum/threads/${thread.id}`;
      }
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发帖失败"),
  });

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) draft.saveNow();
    setOpen(nextOpen);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button disabled={!isAuthenticated}>
          <Plus className="h-4 w-4" />
          发帖
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>发布新帖</DialogTitle>
        </DialogHeader>
        {!isAuthenticated ? (
          <p className="text-sm text-muted-foreground">请先登录再发帖。</p>
        ) : (
          <div className="space-y-3">
            <DraftSyncNotice
              status={draft.status}
              savedAt={draft.savedAt}
              onRestoreRemote={draft.restoreRemote}
              onKeepLocal={draft.keepLocal}
              onRetry={draft.retry}
            />
            <div className="space-y-2">
              <Label>板块</Label>
              <Select value={boardId} onValueChange={setBoardId}>
                <SelectTrigger>
                  <SelectValue placeholder="选择板块" />
                </SelectTrigger>
                <SelectContent>
                  {boards.map((board) => (
                    <SelectItem key={board.id} value={board.id ?? ""} disabled={!board.canPost}>
                      {board.name}
                      {!board.canPost
                        ? board.postingRestriction === "trust_level"
                          ? `（需信任等级 ${board.minTrustToPost}）`
                          : board.postingRestriction === "board_locked"
                            ? "（已锁定）"
                            : "（需登录）"
                        : ""}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {selectedBoard && !selectedBoard.canPost ? (
              <p className="text-sm text-destructive">你当前没有在此板块发帖的权限。</p>
            ) : null}
            <div className="space-y-2">
              <Label>标题</Label>
              <Input value={title} onChange={(event) => setTitle(event.target.value)} maxLength={120} />
            </div>
            <div className="space-y-2">
              <Label>正文</Label>
              <React.Suspense fallback={<p role="status" className="text-sm text-muted-foreground">正在加载编辑器</p>}>
                <MarkdownEditor
                  value={body}
                  onChange={setBody}
                  label="帖子正文"
                  maxLength={50_000}
                  minHeight={240}
                  attachmentUsage="forum_thread"
                  attachmentAssetIds={attachmentAssetIds}
                  onAttachmentAssetIdsChange={setAttachmentAssetIds}
                  maxImages={8}
                  onAttachmentsReadyChange={setAttachmentsReady}
                />
              </React.Suspense>
            </div>
            <div className="space-y-2">
              <Label>标签</Label>
              <Input value={tags} onChange={(event) => setTags(event.target.value)} placeholder="最多 3 个，用空格分隔" />
            </div>
            <div className="space-y-2">
              <Label>投票问题</Label>
              <Input value={pollQuestion} onChange={(event) => setPollQuestion(event.target.value)} placeholder="可选" />
            </div>
            <div className="space-y-2">
              <Label>投票选项</Label>
              <Textarea
                value={pollOptions}
                onChange={(event) => setPollOptions(event.target.value)}
                placeholder="可选，每行一个选项，至少 2 个"
              />
            </div>
          </div>
        )}
        <DialogFooter>
          <Button
            onClick={() => mutation.mutate()}
            disabled={!boardId || !title.trim() || !selectedBoard?.canPost || !attachmentsReady || mutation.isPending}
          >
            <Send className="h-4 w-4" />
            发布
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function ForumPage() {
  const { isAuthenticated } = useAuth();
  const [params, setParams] = useSearchParams();
  const requestedFeed = params.get("feed");
  const selectedFeed = forumFeedOptions.includes(requestedFeed as ForumFeed)
    ? requestedFeed as ForumFeed
    : "hot";
  const feed = !isAuthenticated && authenticatedFeeds.includes(selectedFeed)
    ? "hot"
    : selectedFeed;
  const board = params.get("board") ?? "all";
  const tag = params.get("tag") ?? "all";
  const boards = useQuery({ queryKey: ["forum", "boards"], queryFn: api.boards });
  const tags = useQuery({ queryKey: ["forum", "tags"], queryFn: api.tags });
  const threads = useInfiniteQuery({
    queryKey: ["forum", "threads", feed, board, tag],
    queryFn: ({ pageParam }) =>
      api.threads({
        feed,
        board: board === "all" ? undefined : board,
        tag: tag === "all" ? undefined : tag,
        cursor: pageParam,
      }),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
  });

  function update(next: Record<string, string | null>) {
    const copy = new URLSearchParams(params);
    for (const [key, value] of Object.entries(next)) {
      if (!value || value === "all") {
        copy.delete(key);
      } else {
        copy.set(key, value);
      }
    }
    setParams(copy);
  }

  const boardItems = boards.data ?? [];
  const threadItems = threads.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  useForumDeliveryRefresh(
    threadItems.map((thread) => thread.attachments?.[0]),
    () => void threads.refetch(),
  );

  return (
    <div>
      <PageHeader
        eyebrow="Forum"
        title="你济论坛"
        description="校园公共讨论区。信息流、板块、发帖、评论、投票和通知都接入 Rust v2 forum 域。"
        actions={<CreateThreadDialog boards={boardItems} />}
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_18rem]">
        <Tabs
          value={feed}
          onValueChange={(value) => update({ feed: value })}
          className="gap-0"
        >
          <div className="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <TabsList>
                <TabsTrigger value="hot">热门</TabsTrigger>
                <TabsTrigger value="new">最新</TabsTrigger>
                <TabsTrigger value="following" disabled={!isAuthenticated}>关注</TabsTrigger>
                <TabsTrigger value="subscriptions" disabled={!isAuthenticated}>订阅</TabsTrigger>
                <TabsTrigger value="unread" disabled={!isAuthenticated}>未读</TabsTrigger>
              </TabsList>
            <Select value={board} onValueChange={(value) => update({ board: value })}>
              <SelectTrigger className="w-full md:w-56" aria-label="筛选板块">
                <SelectValue placeholder="全部板块" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部板块</SelectItem>
                {boardItems.map((item) => (
                  <SelectItem key={item.id} value={item.id ?? ""}>
                    {item.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <TabsContent value={feed}>
            {threads.isLoading ? (
              <LoadingState />
            ) : threads.isError ? (
              <ErrorState error={threads.error} onRetry={() => void threads.refetch()} />
            ) : threadItems.length === 0 ? (
              <EmptyState title="还没有帖子" description="切换板块或发布第一条讨论。" />
            ) : (
              <div className="space-y-3">
                {threadItems.map((thread) => (
                  <ThreadCard
                    key={thread.id}
                    thread={thread}
                    boards={boardItems}
                    onAttachmentDeliveryRefresh={() => void threads.refetch()}
                  />
                ))}
                {threads.hasNextPage ? (
                  <Button
                    type="button"
                    variant="outline"
                    className="w-full"
                    disabled={threads.isFetchingNextPage}
                    onClick={() => void threads.fetchNextPage()}
                  >
                    {threads.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
                    {threads.isFetchingNextPage ? "正在加载" : "加载更多帖子"}
                  </Button>
                ) : null}
              </div>
            )}
          </TabsContent>
        </Tabs>

        <aside className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Flame className="h-4 w-4 text-primary" />
                板块
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {boards.isLoading ? (
                <p className="text-sm text-muted-foreground">加载中...</p>
              ) : (
                boardItems.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => update({ board: item.id ?? null })}
                    className="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm transition-colors hover:bg-accent"
                  >
                    <span>{item.name}</span>
                    <span className="text-xs text-muted-foreground">{formatNumber(item.threadCount)}</span>
                  </button>
                ))
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>热门标签</CardTitle>
              <CardDescription>按标签筛选公开讨论。</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-2">
              {tag !== "all" ? (
                <Button size="sm" variant="outline" onClick={() => update({ tag: null })}>
                  清除 #{tag}
                </Button>
              ) : null}
              {(tags.data ?? []).slice(0, 18).map((tag) => (
                <button key={tag.id} type="button" onClick={() => update({ tag: tag.slug ?? null })}>
                  <Badge variant={params.get("tag") === tag.slug ? "default" : "secondary"}>
                    #{tag.name}
                  </Badge>
                </button>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardContent className="flex items-center gap-3 p-4">
              <ThumbsUp className="h-5 w-5 text-primary" />
              <p className="text-sm text-muted-foreground">
                热榜由后端 Redis ZSET 优先，Redis 不可用时回退数据库排序。
              </p>
            </CardContent>
          </Card>
        </aside>
      </div>
    </div>
  );
}

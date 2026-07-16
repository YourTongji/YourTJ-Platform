import {
  Bookmark,
  MessageCircle,
  Share2,
  ThumbsDown,
  ThumbsUp,
} from "lucide-react";
import { Link } from "react-router";
import { toast } from "sonner";

import { PaginatedListState } from "@/components/common/paginated-list-state";
import { EmptyState } from "@/components/common/states";
import { ForumDeliveryImage } from "@/components/content/forum-delivery-image";
import { ForumAuthorAvatar } from "@/components/forum/forum-author-avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ThreadFeed } from "@/lib/api/types";
import { formatNumber, formatRelativeTime } from "@/lib/format";
import { shareForumThread } from "@/lib/forum-share";

export type CommunityFeedMode = "hot" | "new" | "following" | "subscriptions";

function FeedSkeleton() {
  return (
    <div className="space-y-4" aria-label="正在加载社区动态">
      {[0, 1, 2].map((item) => (
        <Card key={item} className="rounded-xl">
          <CardContent className="space-y-4 p-4">
            <div className="flex items-center gap-2">
              <Skeleton className="size-8 rounded-full" />
              <div className="space-y-2">
                <Skeleton className="h-3.5 w-28" />
                <Skeleton className="h-3 w-40" />
              </div>
            </div>
            <Skeleton className="h-5 w-4/5" />
            <div className="space-y-2">
              <Skeleton className="h-3.5 w-full" />
              <Skeleton className="h-3.5 w-2/3" />
            </div>
            <div className="flex gap-5 border-t pt-3">
              <Skeleton className="h-4 w-12" />
              <Skeleton className="h-4 w-12" />
              <Skeleton className="h-4 w-12" />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function PostCard({
  thread,
  onAttachmentDeliveryRefresh,
  onVote,
  onToggleBookmark,
  isVotePending,
  isBookmarkPending,
}: {
  thread: ThreadFeed;
  onAttachmentDeliveryRefresh: () => void;
  onVote: (thread: ThreadFeed, value: "up" | "down") => void;
  onToggleBookmark: (thread: ThreadFeed) => void;
  isVotePending: boolean;
  isBookmarkPending: boolean;
}) {
  const threadUrl = thread.id ? `/forum/threads/${thread.id}` : "/forum";
  const authorHandle = thread.authorHandle || "YourTJ 用户";
  const authorName = thread.authorDisplayName || authorHandle;
  const tag = thread.tags?.[0];
  const share = async () => {
    try {
      const result = await shareForumThread(thread.title || "YourTJ 社区讨论", thread.id);
      if (result === "copied") toast.success("帖子链接已复制");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "分享失败");
    }
  };

  return (
    <Card className="group rounded-xl transition-colors hover:border-primary/25 hover:bg-[#eef1ef] dark:hover:bg-accent/50">
      <CardContent className="p-4">
        <Link to={threadUrl} className="block rounded-lg outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <ForumAuthorAvatar
                avatar={thread.authorAvatar}
                handle={authorHandle}
                onDeliveryRefresh={onAttachmentDeliveryRefresh}
                className="size-8 border-2 border-primary/50 bg-white p-0.5"
                fallbackClassName="text-primary"
              />
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="truncate text-sm font-bold text-foreground">{authorName}</span>
                  {thread.authorDisplayName ? (
                    <span className="truncate text-xs text-muted-foreground">@{authorHandle}</span>
                  ) : null}
                </div>
                <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <span>{formatRelativeTime(thread.lastActivityAt ?? thread.createdAt)}</span>
                  {tag ? (
                    <Badge variant="secondary" className="rounded-full border border-primary/20 bg-primary/10 px-2 text-[10px] text-primary">
                      {tag}
                    </Badge>
                  ) : null}
                </div>
              </div>
            </div>
          </div>

          <h2 className="mt-3 line-clamp-2 text-lg font-semibold leading-7 text-foreground transition-colors group-hover:text-primary">
            {thread.title || "未命名社区讨论"}
          </h2>
          {thread.bodyExcerpt ? (
            <p className="mt-2 line-clamp-2 text-sm leading-6 text-muted-foreground">
              {thread.bodyExcerpt}
            </p>
          ) : null}
        </Link>

        {thread.attachments?.[0] ? (
          <ForumDeliveryImage
            attachment={thread.attachments[0]}
            onDeliveryRefresh={onAttachmentDeliveryRefresh}
            enableLightbox
            loading="lazy"
            decoding="async"
            className="mt-3 max-h-80 w-full rounded-xl border object-cover"
          />
        ) : null}

        <div className="mt-3 flex flex-wrap items-center gap-1 border-t border-border/70 pt-3 text-xs text-muted-foreground">
          <Link
            to={threadUrl}
            className="mr-2 inline-flex h-8 items-center gap-1.5 rounded-md px-2 hover:bg-accent hover:text-foreground"
            aria-label={`${formatNumber(thread.replyCount)} 条回复`}
          >
            <MessageCircle className="size-4" />
            {formatNumber(thread.replyCount)}
          </Link>
          <Button
            type="button"
            variant={thread.viewerVote === "up" ? "secondary" : "ghost"}
            size="sm"
            className="h-8 px-2"
            aria-label={thread.viewerVote === "up" ? "取消赞同" : "赞同"}
            aria-pressed={thread.viewerVote === "up"}
            disabled={isVotePending}
            onClick={() => onVote(thread, "up")}
          >
            <ThumbsUp className="size-4" />
            {formatNumber(thread.voteCount)}
          </Button>
          <Button
            type="button"
            variant={thread.viewerVote === "down" ? "secondary" : "ghost"}
            size="icon"
            className="size-8"
            aria-label={thread.viewerVote === "down" ? "取消反对" : "反对"}
            aria-pressed={thread.viewerVote === "down"}
            disabled={isVotePending}
            onClick={() => onVote(thread, "down")}
          >
            <ThumbsDown className="size-4" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="ml-auto size-8"
            aria-label={`分享：${thread.title}`}
            onClick={() => void share()}
          >
            <Share2 className="size-4" />
          </Button>
          <Button
            type="button"
            variant={thread.isBookmarked ? "secondary" : "ghost"}
            size="icon"
            className="size-8"
            aria-label={thread.isBookmarked ? "取消收藏" : "收藏"}
            aria-pressed={thread.isBookmarked}
            disabled={isBookmarkPending}
            onClick={() => onToggleBookmark(thread)}
          >
            <Bookmark
              className="size-4"
              fill={thread.isBookmarked ? "currentColor" : "none"}
            />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function CommunityFeed({
  mode,
  onModeChange,
  items,
  isLoading,
  error,
  onRetry,
  hasMore,
  isLoadingMore,
  onLoadMore,
  isAuthenticated,
  onAttachmentDeliveryRefresh,
  onVote,
  onToggleBookmark,
  isVotePending,
  isBookmarkPending,
}: {
  mode: CommunityFeedMode;
  onModeChange: (mode: CommunityFeedMode) => void;
  items: ThreadFeed[];
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  onLoadMore?: () => void;
  isAuthenticated: boolean;
  onAttachmentDeliveryRefresh: () => void;
  onVote: (thread: ThreadFeed, value: "up" | "down") => void;
  onToggleBookmark: (thread: ThreadFeed) => void;
  isVotePending?: (thread: ThreadFeed) => boolean;
  isBookmarkPending?: (thread: ThreadFeed) => boolean;
}) {
  return (
    <section aria-label="社区信息流">
      <Tabs
        value={mode}
        onValueChange={(value) => onModeChange(value as CommunityFeedMode)}
        className="gap-0"
      >
        <div className="mb-6 flex h-10 items-start justify-between border-b border-border/50">
          <TabsList className="h-auto gap-4 rounded-none bg-transparent p-0">
            <TabsTrigger
              value="hot"
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              热门
            </TabsTrigger>
            <TabsTrigger
              value="new"
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              最新
            </TabsTrigger>
            <TabsTrigger
              value="following"
              disabled={!isAuthenticated}
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              关注
            </TabsTrigger>
            <TabsTrigger
              value="subscriptions"
              disabled={!isAuthenticated}
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              订阅
            </TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value={mode}>
          <PaginatedListState
            isLoading={isLoading}
            error={error}
            errorTitle="社区动态加载失败"
            isEmpty={items.length === 0}
            onRetry={onRetry}
            hasMore={hasMore}
            isLoadingMore={isLoadingMore}
            onLoadMore={onLoadMore}
            loading={<FeedSkeleton />}
            empty={<EmptyState
              title="还没有社区动态"
              description="去社区发布第一条讨论吧。"
              action={
                <Button asChild size="sm" className="rounded-full px-4">
                  <Link to="/forum">进入社区</Link>
                </Button>
              }
            />}
            loadMoreLabel="加载更多动态"
            buttonClassName="rounded-full"
          >
            <div className="space-y-4">
              {items.map((thread, index) => (
                <PostCard
                  key={thread.id ?? `${thread.title}-${index}`}
                  thread={thread}
                  onAttachmentDeliveryRefresh={onAttachmentDeliveryRefresh}
                  onVote={onVote}
                  onToggleBookmark={onToggleBookmark}
                  isVotePending={isVotePending?.(thread) ?? false}
                  isBookmarkPending={isBookmarkPending?.(thread) ?? false}
                />
              ))}
            </div>
          </PaginatedListState>
        </TabsContent>
      </Tabs>
    </section>
  );
}

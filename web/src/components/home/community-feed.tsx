import {
  MessageCircle,
  ThumbsUp,
} from "lucide-react";
import { Link } from "react-router";

import { EmptyState, ErrorState } from "@/components/common/states";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ThreadFeed } from "@/lib/api/types";
import { formatNumber, formatRelativeTime } from "@/lib/format";

export type CommunityFeedMode = "hot" | "new" | "subscriptions";

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

function PostCard({ thread }: { thread: ThreadFeed }) {
  const threadUrl = thread.id ? `/forum/threads/${thread.id}` : "/forum";
  const author = thread.authorHandle || "YourTJ 用户";
  const tag = thread.tags?.[0];

  return (
    <Card className="group rounded-xl transition-colors hover:border-primary/25 hover:bg-[#eef1ef] dark:hover:bg-accent/50">
      <CardContent className="p-4">
        <Link to={threadUrl} className="block rounded-lg outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-2">
              <Avatar className="size-8 border-2 border-primary/50 bg-white p-0.5">
                <AvatarFallback className="text-xs text-primary">{author.slice(0, 1).toUpperCase()}</AvatarFallback>
              </Avatar>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="truncate text-sm font-bold text-foreground">{author}</span>
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
        </Link>

        <div className="mt-3 flex items-center gap-5 border-t border-border/70 pt-3 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <MessageCircle className="size-4" />
            {formatNumber(thread.replyCount)}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <ThumbsUp className="size-4" />
            {formatNumber(thread.voteCount)}
          </span>
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
  isAuthenticated,
}: {
  mode: CommunityFeedMode;
  onModeChange: (mode: CommunityFeedMode) => void;
  items: ThreadFeed[];
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
  isAuthenticated: boolean;
}) {
  return (
    <section aria-label="社区信息流">
      <div className="mb-6 flex h-10 items-start justify-between border-b border-border/50">
        <Tabs value={mode} onValueChange={(value) => onModeChange(value as CommunityFeedMode)}>
          <TabsList className="h-auto gap-4 rounded-none bg-transparent p-0">
            <TabsTrigger
              value="hot"
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              推荐
            </TabsTrigger>
            <TabsTrigger
              value="new"
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              最新
            </TabsTrigger>
            <TabsTrigger
              value="subscriptions"
              disabled={!isAuthenticated}
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              订阅
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {isLoading ? (
        <FeedSkeleton />
      ) : error ? (
        <ErrorState title="社区动态加载失败" error={error} onRetry={onRetry} />
      ) : items.length === 0 ? (
        <EmptyState
          title="还没有社区动态"
          description="去社区发布第一条讨论吧。"
          action={
            <Button asChild size="sm" className="rounded-full px-4">
              <Link to="/forum">进入社区</Link>
            </Button>
          }
        />
      ) : (
        <div className="space-y-4">
          {items.map((thread, index) => (
            <PostCard key={thread.id ?? `${thread.title}-${index}`} thread={thread} />
          ))}
        </div>
      )}
    </section>
  );
}

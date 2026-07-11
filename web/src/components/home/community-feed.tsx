import {
  Bookmark,
  CalendarDays,
  Gamepad2,
  GraduationCap,
  ListFilter,
  MapPin,
  MessageCircle,
  MoreHorizontal,
  Share2,
  ShoppingBag,
  ThumbsUp,
} from "lucide-react";
import { Link } from "react-router";

import { EmptyState, ErrorState } from "@/components/common/states";
import { TeaBadge } from "@/components/common/tea-badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ThreadFeed } from "@/lib/api/types";
import { formatNumber, formatRelativeTime } from "@/lib/format";

export type CommunityFeedMode = "hot" | "new" | "following";

const discoveryChannels = [
  { label: "嘉定校区", icon: MapPin },
  { label: "四平校区", icon: GraduationCap },
  { label: "选课排课", icon: CalendarDays },
  { label: "闲置交易", icon: ShoppingBag },
  { label: "泛 ACG", icon: Gamepad2 },
] as const;

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

function PostCard({ thread, index }: { thread: ThreadFeed; index: number }) {
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
                  <TeaBadge level={(index % 3) + 2} />
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
            <MoreHorizontal className="size-4 shrink-0 text-muted-foreground" />
          </div>

          <h2 className="mt-3 line-clamp-2 text-lg font-semibold leading-7 text-foreground transition-colors group-hover:text-primary">
            {thread.title || "未命名社区讨论"}
          </h2>
          <p className="mt-1.5 line-clamp-3 text-sm leading-6 text-[#3d4947] dark:text-muted-foreground">
            {thread.tags?.length
              ? `围绕 ${thread.tags.join("、")} 的校园讨论正在进行，点击查看完整内容和最新回复。`
              : "打开帖子查看完整内容、参与讨论并关注后续更新。"}
          </p>
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
          <span className="inline-flex items-center gap-1.5">
            <Bookmark className="size-4" />
            收藏
          </span>
          <span className="ml-auto hidden items-center gap-1.5 sm:inline-flex">
            <Share2 className="size-4" />
            分享
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
}: {
  mode: CommunityFeedMode;
  onModeChange: (mode: CommunityFeedMode) => void;
  items: ThreadFeed[];
  isLoading: boolean;
  error?: unknown;
  onRetry: () => void;
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
              value="following"
              className="h-10 rounded-none border-b-2 border-transparent px-0 pb-3 pt-0 text-sm shadow-none data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              关注
            </TabsTrigger>
          </TabsList>
        </Tabs>
        <Button variant="ghost" size="sm" className="h-8 px-1 text-muted-foreground">
          <ListFilter className="size-3.5" />
          筛选
        </Button>
      </div>

      <nav
        aria-label="热门社区频道"
        className="scrollbar-none mb-4 flex gap-3 overflow-x-auto pb-2"
      >
        {discoveryChannels.map((channel) => (
          <Link
            key={channel.label}
            to={`/forum?tag=${encodeURIComponent(channel.label)}`}
            className="inline-flex h-[38px] shrink-0 items-center gap-2 rounded-full border border-input bg-transparent px-4 text-sm font-medium text-[#3d4947] transition-colors hover:border-primary/40 hover:bg-primary/5 hover:text-primary dark:text-muted-foreground"
          >
            <channel.icon className="size-4 text-primary" strokeWidth={1.8} />
            {channel.label}
          </Link>
        ))}
      </nav>

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
            <PostCard key={thread.id ?? `${thread.title}-${index}`} thread={thread} index={index} />
          ))}
        </div>
      )}
    </section>
  );
}

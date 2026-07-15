import { useInfiniteQuery } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { useForumBookmarkMutation } from "@/components/forum/use-forum-interactions";
import { ProfilePostCard } from "@/components/profile/profile-post-card";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatRelativeTime } from "@/lib/format";
import { forumQueryKeys } from "@/lib/forum-query-keys";

export function BookmarksPage() {
  const { isAuthenticated } = useAuth();
  const bookmarks = useInfiniteQuery({
    queryKey: forumQueryKeys.bookmarks(),
    queryFn: ({ pageParam }) => api.bookmarks(pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: isAuthenticated,
  });
  const bookmark = useForumBookmarkMutation();

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看收藏" />;
  }

  const items = bookmarks.data?.pages.flatMap((page) => page.items ?? []) ?? [];

  return (
    <div>
      <PageHeader title="我的收藏" description="仅你可见的主题和回复收藏。" />
      {bookmarks.isLoading ? (
        <LoadingState />
      ) : bookmarks.isError ? (
        <ErrorState error={bookmarks.error} onRetry={() => void bookmarks.refetch()} />
      ) : items.length === 0 ? (
        <div className="space-y-3">
          <EmptyState title="暂无可见收藏" />
          {bookmarks.hasNextPage ? (
            <Button
              type="button"
              variant="outline"
              className="w-full"
              disabled={bookmarks.isFetchingNextPage}
              onClick={() => void bookmarks.fetchNextPage()}
            >
              {bookmarks.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
              {bookmarks.isFetchingNextPage ? "加载中" : "继续查找较早收藏"}
            </Button>
          ) : null}
        </div>
      ) : (
        <div className="space-y-3">
          {items.map((item) => (
            <ProfilePostCard
              key={`${item.targetType}-${item.targetId}`}
              authorName={item.content.authorDisplayName || item.content.authorHandle}
              authorHandle={item.content.authorHandle}
              post={{
                id: item.content.id,
                title: item.content.title,
                body: item.content.body,
                boardSlug: item.content.boardSlug,
                createdAtLabel: `收藏于 ${formatRelativeTime(item.createdAt)}`,
                replyCount: item.content.replyCount,
                voteCount: item.content.voteCount,
                attachment: item.content.attachments[0],
                href: `/forum/threads/${item.content.threadId}`,
                isBookmarked: true,
              }}
              bookmarkPending={bookmark.isTargetPending({
                id: item.targetId,
                targetType: item.targetType,
              })}
              onToggleBookmark={() => {
                const target = { id: item.targetId, targetType: item.targetType };
                if (!bookmark.isTargetPending(target)) {
                  bookmark.mutate({ ...target, isBookmarked: true });
                }
              }}
              onAttachmentDeliveryRefresh={() => void bookmarks.refetch()}
            />
          ))}
          {bookmarks.hasNextPage ? (
            <Button
              type="button"
              variant="outline"
              className="w-full"
              disabled={bookmarks.isFetchingNextPage}
              onClick={() => void bookmarks.fetchNextPage()}
            >
              {bookmarks.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
              {bookmarks.isFetchingNextPage ? "加载中" : "加载更多收藏"}
            </Button>
          ) : null}
        </div>
      )}
    </div>
  );
}

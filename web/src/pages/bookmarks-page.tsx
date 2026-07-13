import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { ProfilePostCard } from "@/components/profile/profile-post-card";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatRelativeTime } from "@/lib/format";

export function BookmarksPage() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const bookmarks = useInfiniteQuery({
    queryKey: ["forum", "bookmarks"],
    queryFn: ({ pageParam }) => api.bookmarks(pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: isAuthenticated,
  });
  const removeBookmark = useMutation({
    mutationFn: (input: { id: string; targetType: "thread" | "comment" }) => (
      api.removeBookmark(input.id, input.targetType)
    ),
    onSuccess: async () => {
      toast.success("已取消收藏");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["forum", "bookmarks"] }),
        queryClient.invalidateQueries({ queryKey: ["profile"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "取消收藏失败"),
  });

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
              bookmarkPending={removeBookmark.isPending
                && removeBookmark.variables?.id === item.targetId
                && removeBookmark.variables.targetType === item.targetType}
              onToggleBookmark={() => removeBookmark.mutate({
                id: item.targetId,
                targetType: item.targetType,
              })}
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

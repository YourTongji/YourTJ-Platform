import { useQuery } from "@tanstack/react-query";
import { Bookmark } from "lucide-react";
import { Link } from "react-router";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

export function BookmarksPage() {
  const { isAuthenticated } = useAuth();
  const bookmarks = useQuery({
    queryKey: ["forum", "bookmarks"],
    queryFn: () => api.bookmarks(),
    enabled: isAuthenticated,
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后查看收藏" />;
  }

  return (
    <div>
      <PageHeader eyebrow="Forum" title="我的收藏" description="论坛主题和评论收藏。" />
      {bookmarks.isLoading ? (
        <LoadingState />
      ) : bookmarks.isError ? (
        <ErrorState error={bookmarks.error} onRetry={() => void bookmarks.refetch()} />
      ) : (bookmarks.data?.items ?? []).length === 0 ? (
        <EmptyState title="暂无收藏" />
      ) : (
        <div className="space-y-3">
          {bookmarks.data?.items?.map((item) => (
            <Card key={`${item.targetType}-${item.targetId}`}>
              <CardContent className="flex items-center justify-between gap-3 p-4">
                <div>
                  <div className="flex items-center gap-2">
                    <Bookmark className="h-4 w-4 text-primary" />
                    <Badge variant="secondary">{item.targetType}</Badge>
                    <Link
                      to={item.targetType === "thread" ? `/forum/threads/${item.targetId}` : "/forum"}
                      className="font-medium hover:text-primary"
                    >
                      {item.targetId}
                    </Link>
                  </div>
                  {item.note ? <p className="mt-1 text-sm text-muted-foreground">{item.note}</p> : null}
                </div>
                <p className="text-xs text-muted-foreground">{formatUnixTime(item.createdAt)}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

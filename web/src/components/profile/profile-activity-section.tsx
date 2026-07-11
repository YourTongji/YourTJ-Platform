import { Loader2, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ProfileActivitySection({
  title,
  icon: Icon,
  items,
  emptyTitle,
  emptyDescription,
  isLoading,
  error,
  hasMore,
  isLoadingMore,
  onRetry,
  onLoadMore,
}: {
  title: string;
  icon: LucideIcon;
  items: ReactNode[];
  emptyTitle: string;
  emptyDescription: string;
  isLoading: boolean;
  error?: unknown;
  hasMore: boolean;
  isLoadingMore: boolean;
  onRetry: () => void;
  onLoadMore: () => void;
}) {
  return (
    <Card className="h-fit">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icon className="size-4 text-primary" aria-hidden="true" />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <LoadingState label={`加载${title}`} />
        ) : error ? (
          <ErrorState error={error} onRetry={onRetry} />
        ) : items.length === 0 ? (
          <EmptyState
            title={emptyTitle}
            description={emptyDescription}
            className="border-0 bg-muted/20 shadow-none"
          />
        ) : (
          <div className="space-y-2">
            {items}
            {hasMore ? (
              <Button
                type="button"
                variant="outline"
                className="mt-3 w-full"
                onClick={onLoadMore}
                disabled={isLoadingMore}
              >
                {isLoadingMore ? <Loader2 className="size-4 animate-spin" /> : null}
                {isLoadingMore ? "加载中" : `加载更多${title}`}
              </Button>
            ) : null}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

import { Loader2 } from "lucide-react";
import type { ReactNode } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface PaginatedListStateProps {
  children: ReactNode;
  isLoading: boolean;
  error?: unknown;
  isEmpty: boolean;
  onRetry: () => void;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  onLoadMore?: () => void;
  loading?: ReactNode;
  empty?: ReactNode;
  loadMoreLabel?: string;
  loadingMoreLabel?: string;
  errorTitle?: string;
  className?: string;
  buttonClassName?: string;
}

export function PaginatedListState({
  children,
  isLoading,
  error,
  isEmpty,
  onRetry,
  hasMore = false,
  isLoadingMore = false,
  onLoadMore,
  loading = <LoadingState />,
  empty = <EmptyState title="暂无内容" />,
  loadMoreLabel = "加载更多",
  loadingMoreLabel = "正在加载",
  errorTitle,
  className,
  buttonClassName,
}: PaginatedListStateProps) {
  if (isLoading) return loading;
  if (error) return <ErrorState title={errorTitle} error={error} onRetry={onRetry} />;
  if (isEmpty) return empty;

  return (
    <div className={cn("space-y-4", className)} aria-busy={isLoadingMore}>
      {children}
      {hasMore && onLoadMore ? (
        <Button
          type="button"
          variant="outline"
          className={cn("w-full", buttonClassName)}
          disabled={isLoadingMore}
          onClick={onLoadMore}
        >
          {isLoadingMore ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : null}
          {isLoadingMore ? loadingMoreLabel : loadMoreLabel}
        </Button>
      ) : null}
    </div>
  );
}

import { Skeleton } from "@/components/ui/skeleton";

export function RouteLoadingState() {
  return (
    <div role="status" aria-live="polite" className="motion-page space-y-4 p-4 sm:p-6">
      <span className="sr-only">页面加载中</span>
      <Skeleton className="h-7 w-36" />
      <Skeleton className="h-4 w-full max-w-xl" />
      <div className="grid gap-4 pt-2 md:grid-cols-2">
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    </div>
  );
}

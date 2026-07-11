import { useInfiniteQuery } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { Link } from "react-router";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/api/endpoints";

export type ProfileRelationshipListKind = "followers" | "following";

export function ProfileRelationshipListDialog({
  handle,
  kind,
  open,
  onOpenChange,
}: {
  handle: string;
  kind: ProfileRelationshipListKind;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const relationships = useInfiniteQuery({
    queryKey: ["profile", handle, kind],
    queryFn: ({ pageParam }) => kind === "followers"
      ? api.userFollowers(handle, pageParam)
      : api.userFollowing(handle, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: open,
  });
  const items = relationships.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const title = kind === "followers" ? "关注者" : "正在关注";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>列表会应用账号状态、block 和用户可见性设置。</DialogDescription>
        </DialogHeader>
        <div className="max-h-[55vh] space-y-2 overflow-y-auto pr-1">
          {relationships.isLoading ? (
            <p className="py-8 text-center text-sm text-muted-foreground">正在加载…</p>
          ) : relationships.isError ? (
            <div className="rounded-lg border border-destructive/30 p-4 text-sm text-destructive">
              你没有权限查看这个列表，或列表暂时不可用。
            </div>
          ) : items.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">列表还是空的。</p>
          ) : items.map((item) => (
            <Link
              key={item.id}
              to={`/profile/${encodeURIComponent(item.handle)}`}
              onClick={() => onOpenChange(false)}
              className="flex items-center gap-3 rounded-lg border p-3 outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50"
            >
              <Avatar className="size-10">
                <AvatarImage src={item.avatarUrl ?? undefined} alt="" />
                <AvatarFallback>{item.handle.slice(0, 1).toUpperCase()}</AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{item.displayName || item.handle}</p>
                <p className="truncate text-sm text-muted-foreground">@{item.handle}</p>
              </div>
              {item.role !== "user" ? <Badge>{item.role === "admin" ? "管理员" : "版主"}</Badge> : null}
            </Link>
          ))}
          {relationships.hasNextPage ? (
            <Button
              type="button"
              variant="outline"
              className="w-full"
              disabled={relationships.isFetchingNextPage}
              onClick={() => void relationships.fetchNextPage()}
            >
              {relationships.isFetchingNextPage ? <Loader2 className="size-4 animate-spin" /> : null}
              加载更多
            </Button>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}

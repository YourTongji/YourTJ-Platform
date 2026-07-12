import { useQuery } from "@tanstack/react-query";
import { BookOpen, Hash, LayoutGrid, MessageCircle, MessageSquare, Search, UserRound } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";

import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api } from "@/lib/api/endpoints";
import { formatUnixTime } from "@/lib/format";

export function SearchDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [query, setQuery] = React.useState("");
  const trimmed = query.trim();
  const result = useQuery({
    queryKey: ["search", trimmed],
    queryFn: () => api.search(trimmed),
    enabled: trimmed.length >= 2,
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl p-0">
        <DialogHeader className="border-b p-4">
          <DialogTitle>全站搜索</DialogTitle>
          <div className="relative mt-3">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              autoFocus
              aria-label="搜索关键词"
              className="pl-9"
              placeholder="搜索课程、帖子、用户、板块或标签"
            />
          </div>
        </DialogHeader>
        <ScrollArea className="max-h-[60vh] p-4">
          {trimmed.length < 2 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">输入至少 2 个字符开始搜索</p>
          ) : result.isLoading ? (
            <p role="status" className="py-8 text-center text-sm text-muted-foreground">搜索中…</p>
          ) : result.isError ? (
            <div className="space-y-3 py-8 text-center text-sm text-muted-foreground">
              <p>搜索暂时不可用，请稍后重试。</p>
              <button
                type="button"
                className="text-primary underline underline-offset-4"
                onClick={() => void result.refetch()}
              >
                重新搜索
              </button>
            </div>
          ) : (
            <div className="space-y-5">
              <div className="flex justify-end">
                <Link
                  to={`/search?q=${encodeURIComponent(trimmed)}`}
                  onClick={() => onOpenChange(false)}
                  className="text-sm font-medium text-primary underline-offset-4 hover:underline"
                >
                  在完整搜索页查看结果
                </Link>
              </div>
              <section>
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <BookOpen className="h-4 w-4 text-primary" />
                  课程
                </div>
                <div className="space-y-2">
                  {(result.data?.courses ?? []).map((course) => (
                    <Link
                      key={course.id}
                      to={`/courses/${course.id}`}
                      onClick={() => onOpenChange(false)}
                      className="block rounded-md border p-3 transition-colors hover:bg-accent"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <p className="font-medium">{course.name}</p>
                        <Badge variant="secondary">{course.reviewAvg?.toFixed(1) ?? "暂无"} 分</Badge>
                      </div>
                      <p className="mt-1 text-sm text-muted-foreground">
                        {course.code} · {course.teacherName ?? "教师待同步"}
                      </p>
                    </Link>
                  ))}
                  {result.data?.courses?.length === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有课程结果</p>
                  ) : null}
                </div>
              </section>
              <section>
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <MessageSquare className="h-4 w-4 text-primary" />
                  点评
                </div>
                <div className="space-y-2">
                  {(result.data?.reviews ?? []).map((review) => (
                    <Link
                      key={review.id}
                      to={`/courses/${review.courseId}`}
                      onClick={() => onOpenChange(false)}
                      className="block rounded-md border p-3 transition-colors hover:bg-accent"
                    >
                      <p className="line-clamp-2 text-sm">{review.comment ?? "无文字点评"}</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {review.courseName} · {review.rating} 星
                      </p>
                    </Link>
                  ))}
                  {result.data?.reviews?.length === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有点评结果</p>
                  ) : null}
                </div>
              </section>
              <section>
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <MessageCircle className="h-4 w-4 text-primary" />
                  社区帖子
                </div>
                <div className="space-y-2">
                  {(result.data?.threads ?? []).map((thread) => (
                    <Link
                      key={thread.id}
                      to={`/forum/threads/${thread.id}`}
                      onClick={() => onOpenChange(false)}
                      className="block rounded-md border p-3 transition-colors hover:bg-accent"
                    >
                      <p className="font-medium">{thread.title}</p>
                      {thread.bodyExcerpt ? (
                        <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
                          {thread.bodyExcerpt}
                        </p>
                      ) : null}
                      <p className="mt-1 text-xs text-muted-foreground">
                        {thread.authorHandle} · {thread.replyCount} 条回复 · {formatUnixTime(thread.createdAt)}
                      </p>
                    </Link>
                  ))}
                  {result.data?.threads.length === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有帖子结果</p>
                  ) : null}
                </div>
              </section>
              <section>
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <UserRound className="h-4 w-4 text-primary" aria-hidden="true" />
                  用户
                </div>
                <div className="grid gap-2 sm:grid-cols-2">
                  {(result.data?.users ?? []).slice(0, 4).map((user) => (
                    <Link
                      key={user.id}
                      to={`/profile/${encodeURIComponent(user.handle)}`}
                      onClick={() => onOpenChange(false)}
                      className="flex items-center gap-3 rounded-md border p-3 transition-colors hover:bg-accent"
                    >
                      <Avatar className="size-9 border">
                        {user.avatarUrl ? <AvatarImage src={user.avatarUrl} alt="" /> : null}
                        <AvatarFallback>{user.handle.slice(0, 1).toUpperCase()}</AvatarFallback>
                      </Avatar>
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">{user.displayName ?? user.handle}</p>
                        <p className="truncate text-xs text-muted-foreground">@{user.handle}</p>
                      </div>
                    </Link>
                  ))}
                  {result.data?.users.length === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有用户结果</p>
                  ) : null}
                </div>
              </section>
              <section>
                <div className="mb-2 flex items-center gap-4 text-sm font-medium">
                  <span className="inline-flex items-center gap-2">
                    <LayoutGrid className="h-4 w-4 text-primary" aria-hidden="true" />
                    板块
                  </span>
                  <span className="inline-flex items-center gap-2">
                    <Hash className="h-4 w-4 text-primary" aria-hidden="true" />
                    标签
                  </span>
                </div>
                <div className="flex flex-wrap gap-2">
                  {(result.data?.boards ?? []).slice(0, 4).map((board) => (
                    <Link
                      key={`board-${board.id}`}
                      to={`/forum?board=${encodeURIComponent(board.id)}`}
                      onClick={() => onOpenChange(false)}
                      className="rounded-full border px-3 py-1.5 text-sm transition-colors hover:bg-accent"
                    >
                      {board.name}
                    </Link>
                  ))}
                  {(result.data?.tags ?? []).slice(0, 6).map((tag) => (
                    <Link
                      key={`tag-${tag.id}`}
                      to={`/forum?tag=${encodeURIComponent(tag.slug)}`}
                      onClick={() => onOpenChange(false)}
                      className="rounded-full border px-3 py-1.5 text-sm transition-colors hover:bg-accent"
                    >
                      #{tag.name}
                    </Link>
                  ))}
                  {(result.data?.boards.length ?? 0) + (result.data?.tags.length ?? 0) === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有板块或标签结果</p>
                  ) : null}
                </div>
              </section>
            </div>
          )}
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}

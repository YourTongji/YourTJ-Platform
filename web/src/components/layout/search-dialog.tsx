import { useQuery } from "@tanstack/react-query";
import { BookOpen, MessageSquare, Search } from "lucide-react";
import * as React from "react";
import { Link } from "react-router";

import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api } from "@/lib/api/endpoints";

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
              className="pl-9"
              placeholder="搜索课程、老师、点评、帖子"
            />
          </div>
        </DialogHeader>
        <ScrollArea className="max-h-[60vh] p-4">
          {trimmed.length < 2 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">输入至少 2 个字符开始搜索</p>
          ) : result.isLoading ? (
            <p className="py-8 text-center text-sm text-muted-foreground">搜索中...</p>
          ) : (
            <div className="space-y-5">
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
                        {review.authorHandle} · {review.rating} 星
                      </p>
                    </Link>
                  ))}
                  {result.data?.reviews?.length === 0 ? (
                    <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">没有点评结果</p>
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

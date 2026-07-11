import { useQuery } from "@tanstack/react-query";
import { BookOpen, Hash, LayoutGrid, MessageCircle, MessageSquare, Search, SearchX, UserRound } from "lucide-react";
import * as React from "react";
import { Link, useSearchParams } from "react-router";

import { PageHeader } from "@/components/common/page-header";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api/endpoints";
import { formatNumber, formatUnixTime } from "@/lib/format";
import { cn } from "@/lib/utils";

type SearchScope = "all" | "course" | "review" | "thread" | "user" | "board" | "tag";

const scopes: Array<{ value: SearchScope; label: string }> = [
  { value: "all", label: "全部" },
  { value: "course", label: "课程与教师" },
  { value: "review", label: "课评" },
  { value: "thread", label: "社区帖子" },
  { value: "user", label: "用户" },
  { value: "board", label: "板块" },
  { value: "tag", label: "标签" },
];

export function SearchPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const query = searchParams.get("q")?.trim() ?? "";
  const rawScope = searchParams.get("type");
  const scope = scopes.some((item) => item.value === rawScope) ? rawScope as SearchScope : "all";
  const [draft, setDraft] = React.useState(query);
  const result = useQuery({
    queryKey: ["search-page", query, scope],
    queryFn: () => api.search(query, scope, 30),
    enabled: query.length >= 2,
  });

  React.useEffect(() => setDraft(query), [query]);

  function updateSearch(nextQuery: string, nextScope = scope) {
    const next = new URLSearchParams();
    if (nextQuery.trim()) next.set("q", nextQuery.trim());
    if (nextScope !== "all") next.set("type", nextScope);
    setSearchParams(next);
  }

  const courses = result.data?.courses ?? [];
  const reviews = result.data?.reviews ?? [];
  const threads = result.data?.threads ?? [];
  const users = result.data?.users ?? [];
  const boards = result.data?.boards ?? [];
  const tags = result.data?.tags ?? [];
  const total = courses.length + reviews.length + threads.length + users.length + boards.length + tags.length;

  return (
    <div className="mx-auto max-w-4xl">
      <PageHeader
        eyebrow="Search"
        title="全站搜索"
        description="聚合课程、课评、帖子、用户、板块与标签；每条结果都会按当前权限重新验证。"
      />
      <form
        className="relative"
        role="search"
        onSubmit={(event) => {
          event.preventDefault();
          updateSearch(draft);
        }}
      >
        <Search className="pointer-events-none absolute left-4 top-1/2 size-5 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
        <Input
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          aria-label="全站搜索关键词"
          className="h-12 rounded-full pl-12 pr-24"
          placeholder="搜索课程、帖子、用户、板块或标签"
        />
        <Button type="submit" className="absolute right-1.5 top-1/2 h-9 -translate-y-1/2 rounded-full px-5" disabled={draft.trim().length < 2}>
          搜索
        </Button>
      </form>

      <div className="mt-4 flex gap-2 overflow-x-auto pb-1" aria-label="搜索范围">
        {scopes.map((item) => (
          <Button
            key={item.value}
            type="button"
            size="sm"
            variant={scope === item.value ? "default" : "outline"}
            className="shrink-0 rounded-full"
            aria-pressed={scope === item.value}
            onClick={() => updateSearch(query, item.value)}
          >
            {item.label}
          </Button>
        ))}
      </div>

      <div className="mt-6" aria-live="polite">
        {query.length < 2 ? (
          <Card>
            <CardContent className="flex min-h-56 flex-col items-center justify-center gap-3 text-center">
              <Search className="size-8 text-primary" aria-hidden="true" />
              <p className="font-medium">输入至少 2 个字符开始搜索</p>
              <p className="text-sm text-muted-foreground">可以搜索课程、社区内容、用户、板块和标签。</p>
            </CardContent>
          </Card>
        ) : result.isLoading ? (
          <p role="status" className="py-16 text-center text-sm text-muted-foreground">正在聚合搜索结果…</p>
        ) : result.isError ? (
          <Card>
            <CardContent className="flex min-h-56 flex-col items-center justify-center gap-3 text-center">
              <SearchX className="size-8 text-destructive" aria-hidden="true" />
              <p className="font-medium">搜索暂时不可用</p>
              <p className="text-sm text-muted-foreground">没有显示缓存或未经权限复核的结果。</p>
              <Button type="button" variant="outline" onClick={() => void result.refetch()}>重试</Button>
            </CardContent>
          </Card>
        ) : total === 0 ? (
          <Card>
            <CardContent className="flex min-h-56 flex-col items-center justify-center gap-3 text-center">
              <SearchX className="size-8 text-muted-foreground" aria-hidden="true" />
              <p className="font-medium">没有找到“{query}”</p>
              <p className="text-sm text-muted-foreground">试试更短的关键词、课程代码、用户昵称或标签名。</p>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-8">
            <p className="text-sm text-muted-foreground">当前返回 {total} 条经可见性复核的结果</p>

            {(scope === "all" || scope === "course") && courses.length > 0 ? (
              <section aria-labelledby="course-results-title">
                <h2 id="course-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <BookOpen className="size-5 text-primary" aria-hidden="true" />
                  课程与教师
                  <Badge variant="secondary">{courses.length}</Badge>
                </h2>
                <div className="grid gap-3 sm:grid-cols-2">
                  {courses.map((course) => (
                    <Link key={course.id} to={`/courses/${course.id}`} className="rounded-xl border bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <h3 className="truncate font-semibold">{course.name}</h3>
                          <p className="mt-1 text-sm text-muted-foreground">{course.code} · {course.teacherName ?? "教师待同步"}</p>
                        </div>
                        <Badge variant="secondary" className="shrink-0">{course.reviewAvg?.toFixed(1) ?? "暂无"} 分</Badge>
                      </div>
                      <p className="mt-3 text-xs text-muted-foreground">{course.department ?? "院系待同步"} · {course.reviewCount} 条课评</p>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}

            {(scope === "all" || scope === "review") && reviews.length > 0 ? (
              <section aria-labelledby="review-results-title">
                <h2 id="review-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <MessageSquare className="size-5 text-primary" aria-hidden="true" />
                  课评
                  <Badge variant="secondary">{reviews.length}</Badge>
                </h2>
                <div className="space-y-3">
                  {reviews.map((review) => (
                    <Link key={review.id} to={`/courses/${review.courseId}`} className="block rounded-xl border bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50">
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="font-semibold">{review.courseName}</h3>
                        <Badge variant="secondary">{review.rating} 星</Badge>
                      </div>
                      <p className={cn("mt-2 text-sm", review.comment ? "line-clamp-3" : "text-muted-foreground")}>
                        {review.comment ?? "该课评没有文字内容"}
                      </p>
                      <p className="mt-2 text-xs text-muted-foreground">
                        {review.approveCount} 人赞同 · {formatUnixTime(review.createdAt)}
                      </p>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}

            {(scope === "all" || scope === "user") && users.length > 0 ? (
              <section aria-labelledby="user-results-title">
                <h2 id="user-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <UserRound className="size-5 text-primary" aria-hidden="true" />
                  用户
                  <Badge variant="secondary">{users.length}</Badge>
                </h2>
                <div className="grid gap-3 sm:grid-cols-2">
                  {users.map((user) => (
                    <Link
                      key={user.id}
                      to={`/profile/${encodeURIComponent(user.handle)}`}
                      className="flex items-center gap-3 rounded-xl border bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                    >
                      <Avatar className="size-11 border">
                        {user.avatarUrl ? <AvatarImage src={user.avatarUrl} alt="" /> : null}
                        <AvatarFallback>{user.handle.slice(0, 1).toUpperCase()}</AvatarFallback>
                      </Avatar>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <h3 className="truncate font-semibold">{user.displayName ?? user.handle}</h3>
                          {user.following ? <Badge variant="secondary">已关注</Badge> : null}
                        </div>
                        <p className="truncate text-sm text-muted-foreground">@{user.handle}</p>
                        <p className="mt-1 text-xs text-muted-foreground">{formatNumber(user.followerCount)} 位关注者</p>
                      </div>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}

            {(scope === "all" || scope === "thread") && threads.length > 0 ? (
              <section aria-labelledby="thread-results-title">
                <h2 id="thread-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <MessageCircle className="size-5 text-primary" aria-hidden="true" />
                  社区帖子
                  <Badge variant="secondary">{threads.length}</Badge>
                </h2>
                <div className="space-y-3">
                  {threads.map((thread) => (
                    <Link key={thread.id} to={`/forum/threads/${thread.id}`} className="block rounded-xl border bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50">
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="font-semibold">{thread.title}</h3>
                        <Badge variant="outline">{thread.board}</Badge>
                      </div>
                      {thread.bodyExcerpt ? <p className="mt-2 line-clamp-3 text-sm text-muted-foreground">{thread.bodyExcerpt}</p> : null}
                      <p className="mt-2 text-xs text-muted-foreground">
                        {thread.authorHandle} · {thread.replyCount} 条回复 · {formatUnixTime(thread.createdAt)}
                      </p>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}

            {(scope === "all" || scope === "board") && boards.length > 0 ? (
              <section aria-labelledby="board-results-title">
                <h2 id="board-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <LayoutGrid className="size-5 text-primary" aria-hidden="true" />
                  社区板块
                  <Badge variant="secondary">{boards.length}</Badge>
                </h2>
                <div className="grid gap-3 sm:grid-cols-2">
                  {boards.map((board) => (
                    <Link
                      key={board.id}
                      to={`/forum?board=${encodeURIComponent(board.id)}`}
                      className="rounded-xl border bg-card p-4 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                    >
                      <h3 className="font-semibold">{board.name}</h3>
                      <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{board.description ?? "浏览该板块的公开讨论"}</p>
                      <p className="mt-3 text-xs text-muted-foreground">{formatNumber(board.threadCount)} 个帖子</p>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}

            {(scope === "all" || scope === "tag") && tags.length > 0 ? (
              <section aria-labelledby="tag-results-title">
                <h2 id="tag-results-title" className="mb-3 flex items-center gap-2 font-semibold">
                  <Hash className="size-5 text-primary" aria-hidden="true" />
                  标签
                  <Badge variant="secondary">{tags.length}</Badge>
                </h2>
                <div className="flex flex-wrap gap-3">
                  {tags.map((tag) => (
                    <Link
                      key={tag.id}
                      to={`/forum?tag=${encodeURIComponent(tag.slug)}`}
                      className="rounded-full border bg-card px-4 py-2 text-sm transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                    >
                      <span className="font-semibold">#{tag.name}</span>
                      <span className="ml-2 text-muted-foreground">{formatNumber(tag.threadCount)}</span>
                    </Link>
                  ))}
                </div>
              </section>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}

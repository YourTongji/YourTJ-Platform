import { useQuery } from "@tanstack/react-query";
import { Bell, BookOpen, CalendarDays, Flame, MessageSquare, Star, Wallet } from "lucide-react";
import { Link } from "react-router";

import { RatingStars } from "@/components/common/rating-stars";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { TeaBadge } from "@/components/common/tea-badge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { formatDate, formatNumber, formatRating, formatUnixTime } from "@/lib/format";

const quickEntries = [
  { to: "/forum", label: "论坛", icon: MessageSquare },
  { to: "/schedule", label: "排课模拟", icon: CalendarDays },
  { to: "/courses", label: "课程点评", icon: BookOpen },
  { to: "/wallet", label: "积分钱包", icon: Wallet },
  { to: "/notifications", label: "通知", icon: Bell },
];

export function HomePage() {
  const { isAuthenticated, account } = useAuth();
  const threads = useQuery({ queryKey: ["home", "threads"], queryFn: () => api.threads({ feed: "hot" }) });
  const courses = useQuery({ queryKey: ["home", "courses"], queryFn: () => api.courses({ sort: "hot" }) });
  const announcements = useQuery({ queryKey: ["home", "announcements"], queryFn: api.announcements });
  const wallet = useQuery({ queryKey: ["wallet"], queryFn: api.wallet, enabled: isAuthenticated });

  return (
    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_20rem]">
      <div className="space-y-5">
        <section className="rounded-lg border bg-card p-5">
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div>
              <p className="text-sm font-medium text-primary">YourTJ Platform</p>
              <h1 className="mt-1 text-2xl font-semibold">你济社区</h1>
              <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
                论坛、选课、课程点评和闭环积分合并在同一个身份与后端接口中。
              </p>
            </div>
            {isAuthenticated ? (
              <div className="rounded-md bg-secondary px-4 py-3">
                <p className="text-sm text-muted-foreground">欢迎回来</p>
                <div className="mt-1 flex items-center gap-2">
                  <span className="font-semibold">{account?.handle}</span>
                  <TeaBadge level={account?.trustLevel ?? 0} />
                </div>
              </div>
            ) : (
              <Button asChild>
                <Link to="/login">登录参与</Link>
              </Button>
            )}
          </div>
        </section>

        <div className="grid grid-cols-2 gap-3 md:grid-cols-5">
          {quickEntries.map((entry) => (
            <Link key={entry.to} to={entry.to} className="rounded-lg border bg-card p-4 text-center transition-shadow hover:shadow-md">
              <entry.icon className="mx-auto h-5 w-5 text-primary" />
              <p className="mt-2 text-sm font-medium">{entry.label}</p>
            </Link>
          ))}
        </div>

        <section>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="flex items-center gap-2 font-semibold">
              <Flame className="h-4 w-4 text-primary" />
              热门讨论
            </h2>
            <Button asChild variant="ghost" size="sm"><Link to="/forum">全部</Link></Button>
          </div>
          {threads.isLoading ? (
            <LoadingState />
          ) : threads.isError ? (
            <ErrorState error={threads.error} onRetry={() => void threads.refetch()} />
          ) : (threads.data?.items ?? []).length === 0 ? (
            <EmptyState title="暂无讨论" />
          ) : (
            <div className="space-y-3">
              {(threads.data?.items ?? []).slice(0, 5).map((thread) => (
                <Link key={thread.id} to={`/forum/threads/${thread.id}`} className="block rounded-lg border bg-card p-4 transition-shadow hover:shadow-sm">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <h3 className="font-semibold">{thread.title}</h3>
                      <p className="mt-1 text-sm text-muted-foreground">
                        {thread.authorHandle} · {formatUnixTime(thread.lastActivityAt)}
                      </p>
                    </div>
                    <Badge variant="secondary">{formatNumber(thread.replyCount)} 回复</Badge>
                  </div>
                </Link>
              ))}
            </div>
          )}
        </section>

        <section>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="flex items-center gap-2 font-semibold">
              <Star className="h-4 w-4 text-primary" />
              热门课程
            </h2>
            <Button asChild variant="ghost" size="sm"><Link to="/courses">全部</Link></Button>
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            {(courses.data?.items ?? []).slice(0, 4).map((course) => (
              <Link key={course.id} to={`/courses/${course.id}`} className="rounded-lg border bg-card p-4 transition-shadow hover:shadow-sm">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <h3 className="font-semibold">{course.name}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{course.teacherName ?? "教师待同步"}</p>
                  </div>
                  <Badge variant="secondary">{formatRating(course.reviewAvg)}</Badge>
                </div>
                <div className="mt-3 flex items-center gap-2">
                  <RatingStars value={course.reviewAvg ?? 0} />
                  <span className="text-xs text-muted-foreground">{formatNumber(course.reviewCount)} 条</span>
                </div>
              </Link>
            ))}
          </div>
        </section>
      </div>

      <aside className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle>公告</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {(announcements.data ?? []).slice(0, 5).map((item) => (
              <div key={item.id} className="border-b pb-3 last:border-0 last:pb-0">
                <p className="text-sm font-medium">{item.title}</p>
                <p className="mt-1 text-xs text-muted-foreground">{formatDate(item.createdAt)}</p>
              </div>
            ))}
            {!announcements.data?.length ? <p className="text-sm text-muted-foreground">暂无公告</p> : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>钱包</CardTitle>
          </CardHeader>
          <CardContent>
            {isAuthenticated ? (
              <>
                <p className="text-3xl font-semibold text-primary">{formatNumber(wallet.data?.balance)}</p>
                <p className="mt-1 text-sm text-muted-foreground">当前积分余额</p>
                <Button asChild className="mt-4 w-full" variant="secondary"><Link to="/wallet">进入钱包</Link></Button>
              </>
            ) : (
              <p className="text-sm text-muted-foreground">登录后查看积分余额。</p>
            )}
          </CardContent>
        </Card>
      </aside>
    </div>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarPlus, Flag, Heart, Pencil, Send } from "lucide-react";
import * as React from "react";
import { Link, useLocation, useNavigate, useParams, useSearchParams } from "react-router";
import { toast } from "sonner";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { PageHeader } from "@/components/common/page-header";
import { RatingStars } from "@/components/common/rating-stars";
import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
import { ReviewReportDialog } from "@/components/reviews/review-report-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { accountQueryScope } from "@/lib/account-query-keys";
import { api } from "@/lib/api/endpoints";
import type { Page, Review } from "@/lib/api/types";
import { formatDate, formatNumber, formatRating, idempotencyKey } from "@/lib/format";

function ReviewCard({ review, courseId }: { review: Review; courseId: string }) {
  const { account, isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const viewerScope = accountQueryScope(account?.id);
  const navigate = useNavigate();
  const location = useLocation();
  const [reportOpen, setReportOpen] = React.useState(false);
  const [editOpen, setEditOpen] = React.useState(false);
  const [rating, setRating] = React.useState(review.rating ?? 0);
  const [semester, setSemester] = React.useState(review.semester ?? "");
  const [score, setScore] = React.useState(review.score ?? "");
  const [comment, setComment] = React.useState(review.comment ?? "");
  const toggleLike = useMutation({
    mutationFn: () => review.viewerLiked
      ? api.unlikeReview(review.id ?? "")
      : api.likeReview(review.id ?? ""),
    onSuccess: async () => {
      toast.success(review.viewerLiked ? "已取消点赞" : "已点赞");
      await queryClient.invalidateQueries({ queryKey: ["course-reviews", courseId] });
      await queryClient.invalidateQueries({ queryKey: ["review", review.id] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "更新点赞失败"),
  });
  const edit = useMutation({
    mutationFn: () => api.editReview(review.id ?? "", {
      rating,
      semester: semester || undefined,
      score: score || undefined,
      comment: comment || undefined,
    }),
    onMutate: () => ({ viewerScope }),
    onSuccess: async (updated, _variables, context) => {
      setEditOpen(false);
      queryClient.setQueryData(["review", review.id, context.viewerScope], updated);
      toast.success("点评已更新");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["course-reviews", courseId] }),
        queryClient.invalidateQueries({ queryKey: ["course", courseId] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "更新点评失败"),
  });
  const report = useMutation({
    mutationFn: ({ reason, captchaToken }: { reason: string; captchaToken: string }) =>
      api.reportReview(review.id ?? "", reason, captchaToken),
    onSuccess: async () => {
      setReportOpen(false);
      queryClient.setQueriesData<Page<Review>>(
        { queryKey: ["course-reviews", courseId, viewerScope] },
        (current) => current ? {
          ...current,
          items: current.items.map((item) => item.id === review.id
            ? { ...item, canReport: false }
            : item),
        } : current,
      );
      queryClient.setQueryData<Review>(
        ["review", review.id, viewerScope],
        (current) => current ? { ...current, canReport: false } : current,
      );
      toast.success("举报已进入审核队列");
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["course-reviews", courseId, viewerScope],
        }),
        queryClient.invalidateQueries({
          queryKey: ["review", review.id, viewerScope],
        }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "举报失败"),
  });

  function openEditor() {
    setRating(review.rating ?? 0);
    setSemester(review.semester ?? "");
    setScore(review.score ?? "");
    setComment(review.comment ?? "");
    edit.reset();
    setEditOpen(true);
  }

  function requireAccount() {
    if (isAuthenticated) return true;
    toast.error("登录后才能点赞点评");
    const next = `${location.pathname}${location.search}${location.hash}`;
    navigate(`/login?next=${encodeURIComponent(next)}`);
    return false;
  }

  return (
    <>
      <Card id={`review-${review.id}`}>
        <CardContent className="p-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{review.authorHandle}</span>
                <RatingStars value={review.rating} />
                {review.semester ? <Badge variant="outline">{review.semester}</Badge> : null}
              </div>
              <p className="mt-1 text-xs text-muted-foreground">{formatDate(review.createdAt)}</p>
            </div>
            <Badge variant={review.status === "visible" ? "secondary" : "outline"}>{review.status ?? "visible"}</Badge>
          </div>
          <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed">{review.comment || "这条点评没有正文。"}</p>
          {review.score ? <p className="mt-2 text-xs text-muted-foreground">成绩：{review.score}</p> : null}
          <div className="mt-3 flex items-center gap-2">
            {review.canEdit ? (
              <Button type="button" variant="ghost" size="sm" onClick={openEditor}>
                <Pencil className="h-4 w-4" />
                编辑
              </Button>
            ) : (
              <Button
                type="button"
                variant={review.viewerLiked ? "secondary" : "ghost"}
                size="sm"
                aria-label={review.viewerLiked ? "取消点赞" : "点赞"}
                onClick={() => {
                  if (requireAccount()) toggleLike.mutate();
                }}
                disabled={toggleLike.isPending}
              >
                <Heart className={review.viewerLiked ? "h-4 w-4 fill-current" : "h-4 w-4"} />
                {formatNumber(review.approveCount)}
              </Button>
            )}
            {review.canReport ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  report.reset();
                  setReportOpen(true);
                }}
                disabled={report.isPending}
              >
                <Flag className="h-4 w-4" />
                举报
              </Button>
            ) : null}
          </div>
        </CardContent>
      </Card>
      <Dialog open={editOpen} onOpenChange={setEditOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>编辑点评</DialogTitle>
            <DialogDescription>更新评分与课程体验。修改后会重新计算课程评分。</DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-2">
              <Label>评分</Label>
              <RatingStars value={rating} onChange={setRating} size="md" />
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor={`review-${review.id}-semester`}>学期</Label>
                <Input
                  id={`review-${review.id}-semester`}
                  value={semester}
                  onChange={(event) => setSemester(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor={`review-${review.id}-score`}>成绩</Label>
                <Input
                  id={`review-${review.id}-score`}
                  value={score}
                  onChange={(event) => setScore(event.target.value)}
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor={`review-${review.id}-comment`}>正文</Label>
              <Textarea
                id={`review-${review.id}-comment`}
                value={comment}
                onChange={(event) => setComment(event.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setEditOpen(false)} disabled={edit.isPending}>
              取消
            </Button>
            <Button type="button" onClick={() => edit.mutate()} disabled={edit.isPending}>
              {edit.isPending ? "保存中…" : "保存修改"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <ReviewReportDialog
        reviewAuthor={review.authorHandle}
        open={reportOpen}
        isPending={report.isPending}
        error={report.error}
        onOpenChange={setReportOpen}
        onSubmit={(reason, captchaToken) => report.mutate({ reason, captchaToken })}
      />
    </>
  );
}

function ReviewForm({ courseId }: { courseId: string }) {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [rating, setRating] = React.useState(5);
  const [semester, setSemester] = React.useState("");
  const [score, setScore] = React.useState("");
  const [comment, setComment] = React.useState("");
  const [captchaOpen, setCaptchaOpen] = React.useState(false);
  const mutation = useMutation({
    mutationFn: (captchaToken: string) =>
      api.createReview(
        courseId,
        {
          rating,
          semester: semester || undefined,
          score: score || undefined,
          comment: comment || undefined,
          captchaToken,
        },
        idempotencyKey("review"),
      ),
    onSuccess: async () => {
      toast.success("点评已发布");
      setComment("");
      await queryClient.invalidateQueries({ queryKey: ["course-reviews", courseId] });
      await queryClient.invalidateQueries({ queryKey: ["course", courseId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发布失败"),
  });

  if (!isAuthenticated) {
    return <EmptyState title="登录后发布点评" description="点评会绑定账号，但不会公开邮箱。" />;
  }

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>写点评</CardTitle>
          <CardDescription>评分必填，其余字段可选。发布前需要完成人机验证，接口带 Idempotency-Key 防重复提交。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-2">
            <Label>评分</Label>
            <RatingStars value={rating} onChange={setRating} size="md" />
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>学期</Label>
              <Input value={semester} onChange={(event) => setSemester(event.target.value)} placeholder="如 2025 春" />
            </div>
            <div className="space-y-2">
              <Label>成绩</Label>
              <Input value={score} onChange={(event) => setScore(event.target.value)} placeholder="可选" />
            </div>
          </div>
          <div className="space-y-2">
            <Label>正文</Label>
            <Textarea value={comment} onChange={(event) => setComment(event.target.value)} placeholder="课程体验、作业、考核、老师风格..." />
          </div>
          <Button type="button" onClick={() => setCaptchaOpen(true)} disabled={mutation.isPending}>
            <Send className="h-4 w-4" />
            发布点评
          </Button>
        </CardContent>
      </Card>
      <YourTJCaptcha
        open={captchaOpen}
        onOpenChange={setCaptchaOpen}
        onVerified={(captchaToken) => {
          setCaptchaOpen(false);
          mutation.mutate(captchaToken);
        }}
      />
    </>
  );
}

export function CourseDetailPage() {
  const { account } = useAuth();
  const { id } = useParams();
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const courseId = id ?? "";
  const targetReviewId = searchParams.get("review") ?? "";
  const viewerScope = accountQueryScope(account?.id);
  const [reviewSort, setReviewSort] = React.useState<"hot" | "new">("hot");
  const course = useQuery({
    queryKey: ["course", courseId],
    queryFn: () => api.course(courseId),
    enabled: Boolean(courseId),
  });
  const reviews = useQuery({
    queryKey: ["course-reviews", courseId, viewerScope, reviewSort],
    queryFn: () => api.courseReviews(courseId, { sort: reviewSort }),
    enabled: Boolean(courseId),
  });
  const targetReview = useQuery({
    queryKey: ["review", targetReviewId, viewerScope],
    queryFn: () => api.review(targetReviewId),
    enabled: Boolean(courseId && targetReviewId),
    retry: false,
  });
  const related = useQuery({
    queryKey: ["course-related", courseId],
    queryFn: () => api.relatedCourses(courseId),
    enabled: Boolean(courseId),
  });
  const summary = useQuery({
    queryKey: ["course-summary", courseId],
    queryFn: () => api.courseAiSummary(courseId),
    enabled: Boolean(courseId),
    retry: false,
  });

  React.useEffect(() => {
    if (!targetReview.data || location.hash !== `#review-${targetReviewId}`) return;
    document.getElementById(`review-${targetReviewId}`)?.scrollIntoView?.({
      behavior: "smooth",
      block: "center",
    });
  }, [location.hash, targetReview.data, targetReviewId]);

  if (course.isLoading) {
    return <LoadingState label="加载课程详情" />;
  }
  if (course.isError || !course.data) {
    return <ErrorState error={course.error} onRetry={() => void course.refetch()} />;
  }

  const item = course.data;
  const listedReviews = (reviews.data?.items ?? [])
    .filter((review) => review.id !== targetReview.data?.id);
  return (
    <div className="space-y-5">
      <PageHeader
        eyebrow={item.code}
        title={item.name ?? "课程详情"}
        description={`${item.department ?? "院系待同步"} · ${item.teachers?.map((teacher) => teacher.name).join(" / ") || item.teacherName || "教师待同步"}`}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="secondary">{item.credit ?? 0} 学分</Badge>
            {item.code ? (
              <Button asChild variant="outline">
                <Link to={`/schedule?courseCode=${encodeURIComponent(item.code)}`}>
                  <CalendarPlus className="h-4 w-4" />在排课中查看
                </Link>
              </Button>
            ) : null}
          </div>
        }
      />

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="space-y-5">
          <Card>
            <CardContent className="grid gap-4 p-5 sm:grid-cols-3">
              <div>
                <p className="text-sm text-muted-foreground">平均评分</p>
                <div className="mt-2 flex items-center gap-2">
                  <span className="text-3xl font-semibold">{formatRating(item.reviewAvg)}</span>
                  <RatingStars value={item.reviewAvg ?? 0} />
                </div>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">点评数</p>
                <p className="mt-2 text-3xl font-semibold">{formatNumber(item.reviewCount)}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">别名</p>
                <div className="mt-2 flex flex-wrap gap-1">
                  {(item.aliases ?? []).length ? item.aliases?.map((alias) => <Badge key={alias} variant="outline">{alias}</Badge>) : <span className="text-sm text-muted-foreground">暂无</span>}
                </div>
              </div>
            </CardContent>
          </Card>

          {summary.data?.summary ? (
            <Card>
              <CardHeader>
                <CardTitle>AI 点评摘要</CardTitle>
                <CardDescription>
                  {summary.data.model ?? "model"} · {formatDate(summary.data.updatedAt)}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <p className="whitespace-pre-wrap text-sm leading-relaxed">{summary.data.summary}</p>
              </CardContent>
            </Card>
          ) : null}

          <ReviewForm courseId={courseId} />

          {targetReviewId ? (
            <section className="space-y-3" aria-labelledby="target-review-title">
              <h2 id="target-review-title" className="font-semibold">定位点评</h2>
              {targetReview.isLoading ? (
                <LoadingState label="加载指定点评" />
              ) : targetReview.isError ? (
                <ErrorState error={targetReview.error} onRetry={() => void targetReview.refetch()} />
              ) : targetReview.data?.courseId !== courseId ? (
                <EmptyState title="无法定位点评" description="这条点评不属于当前课程。" />
              ) : targetReview.data ? (
                <ReviewCard review={targetReview.data} courseId={courseId} />
              ) : null}
            </section>
          ) : null}

          <section className="space-y-3">
            <div className="flex items-center justify-between gap-3">
              <h2 className="font-semibold">课程点评</h2>
              <Select value={reviewSort} onValueChange={(value) => setReviewSort(value as "hot" | "new")}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="hot">热门</SelectItem>
                  <SelectItem value="new">最新</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {reviews.isLoading ? (
              <LoadingState />
            ) : reviews.isError ? (
              <ErrorState error={reviews.error} onRetry={() => void reviews.refetch()} />
            ) : listedReviews.length === 0 ? (
              <EmptyState title="还没有点评" description="成为第一个记录这门课体验的人。" />
            ) : (
              listedReviews.map((review) => (
                <ReviewCard key={review.id} review={review} courseId={courseId} />
              ))
            )}
          </section>
        </div>

        <aside className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>相关课程</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {(related.data ?? []).length === 0 ? (
                <p className="text-sm text-muted-foreground">暂无相关课程</p>
              ) : (
                related.data?.map((next) => (
                  <a key={next.id} href={`/courses/${next.id}`} className="block rounded-md border p-3 transition-colors hover:bg-accent">
                    <p className="font-medium">{next.name}</p>
                    <p className="text-xs text-muted-foreground">{next.code} · {formatRating(next.reviewAvg)}</p>
                  </a>
                ))
              )}
            </CardContent>
          </Card>
        </aside>
      </div>
    </div>
  );
}

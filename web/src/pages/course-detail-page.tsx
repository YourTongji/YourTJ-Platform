import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Flag, Heart, Send } from "lucide-react";
import * as React from "react";
import { useParams } from "react-router";
import { toast } from "sonner";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { PageHeader } from "@/components/common/page-header";
import { RatingStars } from "@/components/common/rating-stars";
import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
import { ReviewReportDialog } from "@/components/reviews/review-report-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { Review } from "@/lib/api/types";
import { formatDate, formatNumber, formatRating, idempotencyKey } from "@/lib/format";

function ReviewCard({ review, courseId }: { review: Review; courseId: string }) {
  const queryClient = useQueryClient();
  const [reportOpen, setReportOpen] = React.useState(false);
  const like = useMutation({
    mutationFn: () => api.likeReview(review.id ?? ""),
    onSuccess: async () => {
      toast.success("已点赞");
      await queryClient.invalidateQueries({ queryKey: ["course-reviews", courseId] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "点赞失败"),
  });
  const report = useMutation({
    mutationFn: ({ reason, captchaToken }: { reason: string; captchaToken: string }) =>
      api.reportReview(review.id ?? "", reason, captchaToken),
    onSuccess: () => {
      setReportOpen(false);
      toast.success("举报已进入审核队列");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "举报失败"),
  });
  return (
    <>
      <Card>
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
            <Button variant="ghost" size="sm" onClick={() => like.mutate()} disabled={like.isPending}>
              <Heart className="h-4 w-4" />
              {formatNumber(review.approveCount)}
            </Button>
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
          </div>
        </CardContent>
      </Card>
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
  const { id } = useParams();
  const courseId = id ?? "";
  const [reviewSort, setReviewSort] = React.useState<"hot" | "new">("hot");
  const course = useQuery({
    queryKey: ["course", courseId],
    queryFn: () => api.course(courseId),
    enabled: Boolean(courseId),
  });
  const reviews = useQuery({
    queryKey: ["course-reviews", courseId, reviewSort],
    queryFn: () => api.courseReviews(courseId, { sort: reviewSort }),
    enabled: Boolean(courseId),
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

  if (course.isLoading) {
    return <LoadingState label="加载课程详情" />;
  }
  if (course.isError || !course.data) {
    return <ErrorState error={course.error} onRetry={() => void course.refetch()} />;
  }

  const item = course.data;
  return (
    <div className="space-y-5">
      <PageHeader
        eyebrow={item.code}
        title={item.name ?? "课程详情"}
        description={`${item.department ?? "院系待同步"} · ${item.teachers?.map((teacher) => teacher.name).join(" / ") || item.teacherName || "教师待同步"}`}
        actions={<Badge variant="secondary">{item.credit ?? 0} 学分</Badge>}
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
            ) : (reviews.data?.items ?? []).length === 0 ? (
              <EmptyState title="还没有点评" description="成为第一个记录这门课体验的人。" />
            ) : (
              (reviews.data?.items ?? []).map((review) => (
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

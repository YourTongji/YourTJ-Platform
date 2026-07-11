import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, Clock3, FileWarning, KeyRound, Mail, Scale, ShieldAlert } from "lucide-react";
import * as React from "react";
import { Link, useSearchParams } from "react-router";
import { toast } from "sonner";

import { ReasonDialog } from "@/components/admin/admin-primitives";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { YourTJCaptcha } from "@/components/common/yourtj-captcha";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { clearAppealAccess, readAppealAccessToken, writeAppealAccess } from "@/lib/appeal-access";
import { api } from "@/lib/api/endpoints";
import type { Appeal, AppealStatus } from "@/lib/api/types";
import { formatUnixTime, idempotencyKey } from "@/lib/format";

const STATUS_LABELS: Record<AppealStatus, string> = {
  submitted: "已提交",
  in_review: "复核中",
  upheld: "维持原处置",
  overturned: "已撤销原处置",
  amended: "已调整原处置",
  withdrawn: "已撤回",
};

const TARGET_LABELS: Record<string, string> = {
  sanction: "账号制裁",
  forum_thread: "社区主题",
  forum_comment: "社区评论",
  review: "课程评价",
};

function campusEmail(value: string) {
  return value.trim().toLowerCase();
}

function eventIdFromTargetUrl(targetUrl: string) {
  if (!targetUrl.startsWith("/appeals?")) return null;
  const eventId = new URLSearchParams(targetUrl.slice(targetUrl.indexOf("?") + 1)).get("event");
  return eventId && /^\d+$/.test(eventId) ? eventId : null;
}

function AppealAccessCard({ onAuthenticated }: { onAuthenticated: (token: string) => void }) {
  const [mode, setMode] = React.useState<"password" | "code">("password");
  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [code, setCode] = React.useState("");
  const [captchaOpen, setCaptchaOpen] = React.useState(false);
  const passwordLogin = useMutation({
    mutationFn: () => api.appealPasswordLogin({ email: campusEmail(email), password }),
    onSuccess: (result) => {
      writeAppealAccess(result);
      onAuthenticated(result.accessToken);
      toast.success("已进入申诉中心");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "验证失败"),
  });
  const requestCode = useMutation({
    mutationFn: (captchaToken: string) =>
      api.requestEmailCode(campusEmail(email), captchaToken, "appeal"),
    onSuccess: () => toast.success("申诉验证码已发送"),
    onError: (error) => toast.error(error instanceof Error ? error.message : "发送失败"),
  });
  const verifyCode = useMutation({
    mutationFn: () => api.appealEmailVerify({ email: campusEmail(email), code: code.trim() }),
    onSuccess: (result) => {
      writeAppealAccess(result);
      onAuthenticated(result.accessToken);
      toast.success("已进入申诉中心");
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "验证失败"),
  });

  return (
    <Card className="mx-auto max-w-xl">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Scale className="size-5 text-primary" aria-hidden="true" />
          安全进入申诉中心
        </CardTitle>
        <CardDescription>
          即使账号处于封禁状态，也可通过校园邮箱证明身份。这里签发的短期凭据不能访问资料、内容、私信或积分。
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs value={mode} onValueChange={(value) => setMode(value as "password" | "code")}>
          <TabsList className="grid h-auto w-full grid-cols-2">
            <TabsTrigger value="password">密码验证</TabsTrigger>
            <TabsTrigger value="code">邮箱验证码</TabsTrigger>
          </TabsList>
          <div className="mt-5 space-y-4">
            <div className="space-y-2">
              <Label htmlFor="appeal-access-email">同济邮箱</Label>
              <Input
                id="appeal-access-email"
                type="email"
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                placeholder="name@tongji.edu.cn"
              />
            </div>
            <TabsContent value="password" className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="appeal-access-password">密码</Label>
                <Input
                  id="appeal-access-password"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  maxLength={128}
                />
              </div>
              <Button
                className="w-full"
                disabled={!email || !password || passwordLogin.isPending}
                onClick={() => passwordLogin.mutate()}
              >
                <KeyRound className="size-4" aria-hidden="true" />
                {passwordLogin.isPending ? "正在验证" : "验证并进入"}
              </Button>
            </TabsContent>
            <TabsContent value="code" className="space-y-4">
              <div className="grid gap-3 sm:grid-cols-[1fr_auto]">
                <div className="space-y-2">
                  <Label htmlFor="appeal-access-code">验证码</Label>
                  <Input
                    id="appeal-access-code"
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    value={code}
                    onChange={(event) => setCode(event.target.value.replace(/\D/g, "").slice(0, 6))}
                  />
                </div>
                <Button
                  type="button"
                  variant="outline"
                  className="self-end"
                  disabled={!email || requestCode.isPending}
                  onClick={() => setCaptchaOpen(true)}
                >
                  <Mail className="size-4" aria-hidden="true" />
                  发送验证码
                </Button>
              </div>
              <Button
                className="w-full"
                disabled={!email || code.length !== 6 || verifyCode.isPending}
                onClick={() => verifyCode.mutate()}
              >
                {verifyCode.isPending ? "正在验证" : "验证并进入"}
              </Button>
            </TabsContent>
          </div>
        </Tabs>
      </CardContent>
      <YourTJCaptcha
        open={captchaOpen}
        onOpenChange={setCaptchaOpen}
        onVerified={(token) => {
          setCaptchaOpen(false);
          requestCode.mutate(token);
        }}
      />
    </Card>
  );
}

function AppealCard({ appeal, onWithdraw }: { appeal: Appeal; onWithdraw: (appeal: Appeal) => void }) {
  return (
    <Card id={`appeal-${appeal.id}`}>
      <CardContent className="space-y-4 p-4 sm:p-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <Badge>{STATUS_LABELS[appeal.status]}</Badge>
              <Badge variant="outline">{TARGET_LABELS[appeal.targetKind] ?? appeal.targetKind}</Badge>
              <span className="text-xs text-muted-foreground">事件 #{appeal.governanceEventId}</span>
            </div>
            <p className="mt-2 text-sm font-medium">{appeal.originalReason ?? "处置原因未提供公开摘要"}</p>
            <p className="mt-1 text-sm text-muted-foreground">你的申诉：{appeal.submissionReason}</p>
          </div>
          {appeal.status === "submitted" ? (
            <Button type="button" variant="outline" size="sm" onClick={() => onWithdraw(appeal)}>
              撤回申诉
            </Button>
          ) : null}
        </div>
        <ol className="space-y-3 border-l border-border pl-4" aria-label="申诉状态历史">
          {appeal.history.map((event) => (
            <li key={event.id} className="relative text-sm">
              <span className="absolute -left-[21px] top-1.5 size-2 rounded-full bg-primary" aria-hidden="true" />
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{STATUS_LABELS[event.toStatus]}</span>
                <span className="text-xs text-muted-foreground">{formatUnixTime(event.createdAt)}</span>
              </div>
              <p className="mt-1 text-muted-foreground">{event.reason}</p>
            </li>
          ))}
        </ol>
      </CardContent>
    </Card>
  );
}

export function AppealsPage() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const [appealToken, setAppealToken] = React.useState(() => readAppealAccessToken());
  const [eventId, setEventId] = React.useState(searchParams.get("event") ?? "");
  const [reason, setReason] = React.useState("");
  const [withdrawTarget, setWithdrawTarget] = React.useState<Appeal | null>(null);
  const canAccess = isAuthenticated || Boolean(appealToken);
  const customToken = isAuthenticated ? undefined : appealToken ?? undefined;
  const appeals = useInfiniteQuery({
    queryKey: ["appeals", customToken ? "restricted" : "session"],
    queryFn: ({ pageParam }) => api.myAppeals(pageParam, customToken),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: canAccess,
  });
  const governanceNotices = useInfiniteQuery({
    queryKey: ["governance-notices", { surface: "appeals", access: customToken ? "restricted" : "session" }],
    queryFn: ({ pageParam }) => api.governanceNotices(undefined, pageParam, customToken),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.hasMore ? page.nextCursor ?? undefined : undefined,
    enabled: canAccess,
  });
  const submit = useMutation({
    mutationFn: () => api.submitAppeal(
      { governanceEventId: eventId.trim(), reason: reason.trim() },
      idempotencyKey("appeal"),
      customToken,
    ),
    onSuccess: async () => {
      toast.success("申诉已提交");
      setReason("");
      await queryClient.invalidateQueries({ queryKey: ["appeals"] });
      await queryClient.invalidateQueries({ queryKey: ["governance-notices"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "提交失败"),
  });
  const withdraw = useMutation({
    mutationFn: ({ appeal, reason }: { appeal: Appeal; reason: string }) =>
      api.withdrawAppeal(appeal.id, appeal.version, reason, customToken),
    onSuccess: async () => {
      setWithdrawTarget(null);
      toast.success("申诉已撤回");
      await queryClient.invalidateQueries({ queryKey: ["appeals"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "撤回失败"),
  });
  const markNoticeRead = useMutation({
    mutationFn: (id: string) => api.markGovernanceNoticesRead([id], customToken),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["governance-notices"] });
      await queryClient.invalidateQueries({ queryKey: ["governance-notice-count"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "标记已读失败"),
  });

  const items = appeals.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const noticeItems = governanceNotices.data?.pages.flatMap((page) => page.items ?? []) ?? [];

  return (
    <div>
      <PageHeader
        eyebrow="Governance"
        title="申诉中心"
        description="申诉关联原处置事件，原记录不会被覆盖；复核人与原处置人必须分离。"
        actions={canAccess && !isAuthenticated ? (
          <Button
            variant="outline"
            onClick={() => {
              clearAppealAccess();
              setAppealToken(null);
            }}
          >
            退出申诉访问
          </Button>
        ) : undefined}
      />

      {!canAccess ? (
        <AppealAccessCard onAuthenticated={setAppealToken} />
      ) : (
        <div className="space-y-5">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <ShieldAlert className="size-5 text-primary" aria-hidden="true" />
                处置与申诉通知
              </CardTitle>
              <CardDescription>
                这里仅显示与你相关的安全摘要；不会公开举报人、工作人员身份或私密证据。
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {governanceNotices.isLoading ? (
                <LoadingState label="加载治理通知" />
              ) : governanceNotices.isError ? (
                <ErrorState
                  error={governanceNotices.error}
                  onRetry={() => void governanceNotices.refetch()}
                />
              ) : noticeItems.length === 0 ? (
                <p className="text-sm text-muted-foreground">暂无治理通知。</p>
              ) : (
                <>
                  {noticeItems.map((notice) => {
                    const noticeEventId = eventIdFromTargetUrl(notice.targetUrl);
                    return (
                      <div
                        key={notice.id}
                        className="flex flex-col gap-3 rounded-lg border p-3 sm:flex-row sm:items-center"
                      >
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <p className="text-sm font-medium">{notice.summary}</p>
                            {!notice.read ? <Badge>未读</Badge> : null}
                          </div>
                          <p className="mt-1 text-xs text-muted-foreground">
                            {formatUnixTime(notice.createdAt)}
                          </p>
                        </div>
                        <div className="flex shrink-0 flex-wrap gap-2">
                          {noticeEventId ? (
                            <Button
                              type="button"
                              size="sm"
                              onClick={() => {
                                setEventId(noticeEventId);
                                if (!notice.read) markNoticeRead.mutate(notice.id);
                              }}
                            >
                              用此事件申诉
                            </Button>
                          ) : (
                            <Button asChild type="button" size="sm" variant="outline">
                              <Link
                                to={notice.targetUrl}
                                onClick={() => !notice.read && markNoticeRead.mutate(notice.id)}
                              >
                                查看申诉状态
                              </Link>
                            </Button>
                          )}
                          {!notice.read ? (
                            <Button
                              type="button"
                              size="icon"
                              variant="ghost"
                              aria-label="标记治理通知为已读"
                              disabled={markNoticeRead.isPending}
                              onClick={() => markNoticeRead.mutate(notice.id)}
                            >
                              <Check className="size-4" aria-hidden="true" />
                            </Button>
                          ) : null}
                        </div>
                      </div>
                    );
                  })}
                  {governanceNotices.hasNextPage ? (
                    <div className="flex justify-center">
                      <Button
                        type="button"
                        variant="outline"
                        disabled={governanceNotices.isFetchingNextPage}
                        onClick={() => void governanceNotices.fetchNextPage()}
                      >
                        {governanceNotices.isFetchingNextPage ? "加载中" : "加载更多通知"}
                      </Button>
                    </div>
                  ) : null}
                </>
              )}
            </CardContent>
          </Card>

          <Card className="border-primary/30 bg-primary/[0.03]">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <FileWarning className="size-5 text-primary" aria-hidden="true" />
                对一项处置提出申诉
              </CardTitle>
              <CardDescription>
                处置通知会带入事件编号。每项处置只能申诉一次，须在通知后的 30 天内提交。
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 md:grid-cols-[180px_1fr_auto] md:items-end">
              <div className="space-y-2">
                <Label htmlFor="appeal-event-id">治理事件编号</Label>
                <Input
                  id="appeal-event-id"
                  inputMode="numeric"
                  value={eventId}
                  onChange={(event) => setEventId(event.target.value.replace(/\D/g, ""))}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="appeal-reason">申诉理由</Label>
                <Textarea
                  id="appeal-reason"
                  value={reason}
                  onChange={(event) => setReason(event.target.value)}
                  maxLength={1000}
                  placeholder="说明你认为处置需要复核的具体事实，不要填写他人的隐私信息。"
                />
              </div>
              <Button
                disabled={!eventId || reason.trim().length < 3 || submit.isPending}
                onClick={() => submit.mutate()}
              >
                {submit.isPending ? "提交中" : "提交申诉"}
              </Button>
            </CardContent>
          </Card>

          {appeals.isLoading ? (
            <LoadingState label="加载申诉记录" />
          ) : appeals.isError ? (
            <ErrorState error={appeals.error} onRetry={() => void appeals.refetch()} />
          ) : items.length === 0 ? (
            <EmptyState title="还没有申诉记录" description="如收到可申诉的治理通知，可使用通知中的事件编号提交。" />
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Clock3 className="size-4" aria-hidden="true" />
                状态历史按时间保留，任何决定都不会静默覆盖原处置。
              </div>
              {items.map((appeal) => (
                <AppealCard key={appeal.id} appeal={appeal} onWithdraw={setWithdrawTarget} />
              ))}
              {appeals.hasNextPage ? (
                <div className="flex justify-center">
                  <Button
                    variant="outline"
                    disabled={appeals.isFetchingNextPage}
                    onClick={() => void appeals.fetchNextPage()}
                  >
                    {appeals.isFetchingNextPage ? "加载中" : "加载更多"}
                  </Button>
                </div>
              ) : null}
            </div>
          )}
        </div>
      )}

      <ReasonDialog
        open={Boolean(withdrawTarget)}
        onOpenChange={(open) => !open && setWithdrawTarget(null)}
        title="撤回这项申诉？"
        description="只有尚未被工作人员领取的申诉可以撤回。撤回会保留历史，原处置不会改变。"
        confirmLabel="确认撤回"
        isPending={withdraw.isPending}
        onConfirm={(withdrawReason) => {
          if (withdrawTarget) withdraw.mutate({ appeal: withdrawTarget, reason: withdrawReason });
        }}
      />
    </div>
  );
}

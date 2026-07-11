import { useMutation, useQuery } from "@tanstack/react-query";
import { CheckCircle2, Eye, ShieldCheck, UserRound } from "lucide-react";
import * as React from "react";
import { Link, Navigate, useNavigate } from "react-router";
import { toast } from "sonner";

import { RouteLoadingState } from "@/components/common/route-loading-state";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { ActivityVisibility, ProfileVisibility } from "@/lib/api/types";

const visibilityOptions = [
  { value: "public", label: "所有人", description: "校外访客也可以看到" },
  { value: "campus", label: "仅校园用户", description: "只有已登录的校园用户可以看到" },
  { value: "only_me", label: "仅自己", description: "不在公开资料中展示" },
] as const;

function visibilityDescription(value: ProfileVisibility | ActivityVisibility) {
  return visibilityOptions.find((option) => option.value === value)?.description ?? "";
}

export function OnboardingPage() {
  const navigate = useNavigate();
  const { account, isAuthenticated, isLoading, refreshMe } = useAuth();
  const onboarding = useQuery({
    queryKey: ["onboarding"],
    queryFn: api.onboarding,
    enabled: isAuthenticated,
    staleTime: 0,
  });
  const [handle, setHandle] = React.useState("");
  const [displayName, setDisplayName] = React.useState("");
  const [bio, setBio] = React.useState("");
  const [profileVisibility, setProfileVisibility] = React.useState<ProfileVisibility>("campus");
  const [activityVisibility, setActivityVisibility] = React.useState<ActivityVisibility>("campus");
  const [discoverable, setDiscoverable] = React.useState(true);
  const [accepted, setAccepted] = React.useState(false);
  const initialized = React.useRef(false);

  React.useEffect(() => {
    if (!onboarding.data || initialized.current) return;
    initialized.current = true;
    setHandle(onboarding.data.handle);
    setDisplayName(onboarding.data.displayName ?? "");
    setBio(onboarding.data.bio ?? "");
    setProfileVisibility(onboarding.data.profileVisibility);
    setActivityVisibility(onboarding.data.activityVisibility);
    setDiscoverable(onboarding.data.discoverable);
  }, [onboarding.data]);

  const complete = useMutation({
    mutationFn: async () => {
      const state = onboarding.data;
      if (!state) throw new Error("入门信息尚未加载");
      return api.completeOnboarding({
        handle: handle.trim().toLowerCase(),
        displayName: displayName.trim() || null,
        bio: bio.trim() || null,
        profileVisibility,
        activityVisibility,
        discoverable,
        acceptedTermsVersion: state.currentTermsVersion,
      });
    },
    onSuccess: async () => {
      await refreshMe();
      toast.success("设置已保存，欢迎来到 YourTJ");
      navigate("/", { replace: true });
    },
  });

  if (isLoading) return <RouteLoadingState />;
  if (!isAuthenticated) return <Navigate to="/login?next=/onboarding" replace />;
  if (!account?.onboardingRequired && !onboarding.data?.required) {
    return <Navigate to="/" replace />;
  }

  const normalizedHandle = handle.trim().toLowerCase();
  const handleIsValid = /^[a-z0-9._-]{3,30}$/.test(normalizedHandle);
  const error = complete.error ?? onboarding.error;

  return (
    <div className="mx-auto max-w-3xl py-4 sm:py-10">
      <div className="mb-7 max-w-2xl">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">First run</p>
        <h1 className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">先把你的社区身份设置好</h1>
        <p className="mt-3 leading-7 text-muted-foreground">
          邮箱只用于校园身份和账号安全。你可以单独决定资料与活动的可见范围，之后也能在设置中修改。
        </p>
      </div>

      {onboarding.isLoading ? <RouteLoadingState /> : onboarding.isError ? (
        <Card role="alert">
          <CardContent className="space-y-3 pt-5">
            <p className="text-sm text-destructive">无法读取入门设置，请检查网络后重试。</p>
            <Button variant="outline" onClick={() => void onboarding.refetch()}>重新加载</Button>
          </CardContent>
        </Card>
      ) : (
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (accepted) complete.mutate();
          }}
        >
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><UserRound className="size-5 text-primary" aria-hidden="true" />公开身份</CardTitle>
              <CardDescription>handle 用于个人主页地址；请不要使用学号、邮箱或真实姓名。</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="onboarding-handle">公开 handle</Label>
                <Input
                  id="onboarding-handle"
                  value={handle}
                  onChange={(event) => setHandle(event.target.value.toLowerCase())}
                  autoComplete="username"
                  minLength={3}
                  maxLength={30}
                  pattern="[a-z0-9._-]{3,30}"
                  aria-describedby="onboarding-handle-help"
                  required
                />
                <p id="onboarding-handle-help" className="text-xs leading-5 text-muted-foreground">3–30 位小写字母、数字、点、短横线或下划线。</p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="onboarding-display-name">显示名称（可选）</Label>
                <Input
                  id="onboarding-display-name"
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                  maxLength={50}
                  autoComplete="name"
                  placeholder="你希望大家如何称呼你"
                />
              </div>
              <div className="space-y-2 sm:col-span-2">
                <Label htmlFor="onboarding-bio">简介（可选）</Label>
                <Textarea
                  id="onboarding-bio"
                  value={bio}
                  onChange={(event) => setBio(event.target.value)}
                  maxLength={500}
                  rows={4}
                  placeholder="专业方向、兴趣或你愿意公开分享的内容"
                />
                <p className="text-right text-xs text-muted-foreground">{bio.length}/500</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><Eye className="size-5 text-primary" aria-hidden="true" />默认隐私</CardTitle>
              <CardDescription>公开帖子仍按其所在板块规则展示；这里控制个人资料页与活动列表。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="profile-visibility">个人资料可见范围</Label>
                  <Select value={profileVisibility} onValueChange={(value) => setProfileVisibility(value as ProfileVisibility)}>
                    <SelectTrigger id="profile-visibility"><SelectValue /></SelectTrigger>
                    <SelectContent>{visibilityOptions.map((option) => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}</SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">{visibilityDescription(profileVisibility)}</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="activity-visibility">活动列表可见范围</Label>
                  <Select value={activityVisibility} onValueChange={(value) => setActivityVisibility(value as ActivityVisibility)}>
                    <SelectTrigger id="activity-visibility"><SelectValue /></SelectTrigger>
                    <SelectContent>{visibilityOptions.map((option) => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}</SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">{visibilityDescription(activityVisibility)}</p>
                </div>
              </div>
              <div className="flex items-start justify-between gap-4 rounded-lg border bg-muted/20 p-4">
                <div>
                  <Label htmlFor="onboarding-discoverable">允许被搜索发现</Label>
                  <p className="mt-1 text-xs leading-5 text-muted-foreground">关闭后，你不会出现在用户搜索和推荐中；已有公开内容仍可按内容规则访问。</p>
                </div>
                <Switch id="onboarding-discoverable" checked={discoverable} onCheckedChange={setDiscoverable} aria-label="允许被搜索发现" />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><ShieldCheck className="size-5 text-primary" aria-hidden="true" />规则与数据边界</CardTitle>
              <CardDescription>当前版本：{onboarding.data?.currentTermsVersion}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="rounded-lg border bg-muted/20 p-4 text-sm leading-6 text-muted-foreground">
                <ul className="list-disc space-y-1 pl-5">
                  <li>尊重他人，不发布骚扰、欺诈、违法或泄露个人信息的内容。</li>
                  <li>公开内容可能被审核、搜索和长期保留；私信并非端到端加密。</li>
                  <li>积分是封闭社区权益，不支持充值、提现或自由转账。</li>
                  <li>你可以导出数据、停用账号，或申请删除并在 30 天内恢复。</li>
                </ul>
              </div>
              <label className="flex cursor-pointer items-start gap-3 rounded-lg border p-4 focus-within:ring-[3px] focus-within:ring-ring/50">
                <input
                  type="checkbox"
                  checked={accepted}
                  onChange={(event) => setAccepted(event.target.checked)}
                  className="mt-1 size-4 accent-primary"
                  required
                />
                <span className="text-sm leading-6">我已阅读并同意上述当前版本规则与数据边界，并理解公开内容、私信和账号删除的处理方式。</span>
              </label>
            </CardContent>
          </Card>

          {error ? <p className="text-sm text-destructive" role="alert">{error instanceof Error ? error.message : "保存失败，请重试"}</p> : null}
          <div className="flex flex-col gap-3 rounded-xl bg-primary/[0.06] p-4 sm:flex-row sm:items-center sm:justify-between">
            <p className="flex items-center gap-2 text-sm text-muted-foreground"><CheckCircle2 className="size-4 text-primary" aria-hidden="true" />完成后即可发帖、评论和使用社区功能。</p>
            <Button type="submit" size="lg" disabled={!handleIsValid || !accepted || complete.isPending}>
              {complete.isPending ? "正在保存…" : "保存并进入社区"}
            </Button>
          </div>
        </form>
      )}
      <p className="mt-5 text-center text-sm text-muted-foreground">
        暂时不继续？你仍可
        <Button asChild variant="link" className="h-auto px-1"><Link to="/settings">管理账号安全、导出或关闭账号</Link></Button>
      </p>
    </div>
  );
}

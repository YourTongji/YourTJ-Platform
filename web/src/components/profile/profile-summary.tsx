import {
  Ban,
  Heart,
  MessageCircle,
  MessageSquare,
  Settings,
  ShieldCheck,
  Sparkles,
  UserRoundCheck,
} from "lucide-react";
import { Link } from "react-router";

import { TeaBadge } from "@/components/common/tea-badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { UserProfile } from "@/lib/api/types";
import { formatDate } from "@/lib/format";

const roleLabels = {
  user: "社区成员",
  mod: "社区版主",
  admin: "平台管理员",
} as const;

interface ProfileSummaryProps {
  profile: UserProfile;
  isAuthenticated: boolean;
  isSelf: boolean;
  isIgnored: boolean;
  relationshipLoading: boolean;
  relationshipPending: boolean;
  messagePending: boolean;
  canStartConversation: boolean;
  canManageUser: boolean;
  confirmBlockOpen: boolean;
  onConfirmBlockOpenChange: (open: boolean) => void;
  onStartConversation: () => void;
  onToggleIgnore: () => void;
}

function Stat({ icon: Icon, label, value }: { icon: typeof MessageSquare; label: string; value: number }) {
  return (
    <div className="rounded-lg border bg-muted/25 px-3 py-2.5">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Icon className="size-3.5" aria-hidden="true" />
        {label}
      </div>
      <p className="mt-1 text-lg font-semibold tabular-nums">{value}</p>
    </div>
  );
}

export function ProfileSummary({
  profile,
  isAuthenticated,
  isSelf,
  isIgnored,
  relationshipLoading,
  relationshipPending,
  messagePending,
  canStartConversation,
  canManageUser,
  confirmBlockOpen,
  onConfirmBlockOpenChange,
  onStartConversation,
  onToggleIgnore,
}: ProfileSummaryProps) {
  const roleLabel = roleLabels[profile.role];

  return (
    <>
      <Card className="overflow-hidden">
        <div className="h-20 bg-gradient-to-r from-primary/15 via-primary/5 to-transparent" />
        <CardContent className="-mt-8 grid gap-5 p-5 pt-0 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <div className="min-w-0">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-end">
              <Avatar className="size-20 border-4 border-card shadow-sm">
                <AvatarImage src={profile.avatarUrl ?? undefined} alt={`${profile.handle} 的头像`} />
                <AvatarFallback className="text-xl">
                  {profile.handle.slice(0, 1).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-1 pb-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="truncate text-xl font-semibold">{profile.handle}</h2>
                  <TeaBadge level={profile.trustLevel} />
                  <Badge variant={profile.role === "user" ? "secondary" : "default"}>
                    {roleLabel}
                  </Badge>
                </div>
                <p className="mt-1 text-sm text-muted-foreground">
                  {formatDate(profile.createdAt)} 加入 YourTJ
                </p>
              </div>
            </div>

            <div className="mt-4 flex min-h-7 flex-wrap items-center gap-2" aria-label="用户徽章">
              <Sparkles className="size-4 text-primary" aria-hidden="true" />
              {profile.badges.length > 0 ? (
                profile.badges.map((badge) => (
                  <Badge key={badge.slug} variant="outline" title={badge.slug}>
                    {badge.name}
                  </Badge>
                ))
              ) : (
                <span className="text-sm text-muted-foreground">尚未获得社区徽章</span>
              )}
            </div>

            <div className="mt-5 flex flex-wrap gap-2">
              {isSelf ? (
                <Button asChild variant="outline">
                  <Link to="/settings"><Settings className="size-4" />编辑资料</Link>
                </Button>
              ) : isAuthenticated ? (
                <>
                  <Button
                    type="button"
                    onClick={onStartConversation}
                    disabled={!canStartConversation || messagePending || isIgnored}
                  >
                    <MessageSquare className="size-4" />
                    {isIgnored ? "解除屏蔽后私信" : "发私信"}
                  </Button>
                  <Button
                    type="button"
                    variant={isIgnored ? "outline" : "destructive"}
                    onClick={() => isIgnored ? onToggleIgnore() : onConfirmBlockOpenChange(true)}
                    disabled={relationshipLoading || relationshipPending}
                  >
                    {isIgnored ? <UserRoundCheck className="size-4" /> : <Ban className="size-4" />}
                    {isIgnored ? "解除屏蔽" : "屏蔽用户"}
                  </Button>
                </>
              ) : (
                <Button asChild>
                  <Link to="/login"><MessageSquare className="size-4" />登录后私信</Link>
                </Button>
              )}

              {canManageUser && !isSelf ? (
                <Button asChild variant="secondary">
                  <Link to={`/admin?section=users&q=${encodeURIComponent(profile.handle)}`}>
                    <ShieldCheck className="size-4" />用户治理
                  </Link>
                </Button>
              ) : null}
            </div>
            {isAuthenticated && !isSelf && !canStartConversation && !isIgnored ? (
              <p className="mt-2 text-xs text-muted-foreground">达到信任等级 1 后可发起新私信。</p>
            ) : null}
            {isIgnored ? (
              <p className="mt-2 text-xs text-muted-foreground">
                已屏蔽该用户；双方将无法互发私信，其内容也会从你的社区信息流中隐藏。
              </p>
            ) : null}
          </div>

          <div className="grid grid-cols-3 gap-2 self-end" aria-label="公开社区数据">
            <Stat icon={MessageSquare} label="主题" value={profile.threadCount} />
            <Stat icon={MessageCircle} label="回复" value={profile.commentCount} />
            <Stat icon={Heart} label="获赞" value={profile.votesReceived} />
          </div>
        </CardContent>
      </Card>

      <Dialog open={confirmBlockOpen} onOpenChange={onConfirmBlockOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>屏蔽 {profile.handle}？</DialogTitle>
            <DialogDescription>
              屏蔽后双方不能互发私信，你也不会再在社区信息流中看到该用户的内容。之后可以随时解除。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onConfirmBlockOpenChange(false)}>
              取消
            </Button>
            <Button type="button" variant="destructive" onClick={onToggleIgnore} disabled={relationshipPending}>
              <Ban className="size-4" />确认屏蔽
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

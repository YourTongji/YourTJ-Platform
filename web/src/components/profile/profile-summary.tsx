import {
  Ban,
  CalendarDays,
  Link2,
  MapPin,
  MessageSquare,
  Settings,
  ShieldCheck,
  UserCheck,
  UserPlus,
  UserRoundCheck,
  Volume2,
  VolumeX,
} from "lucide-react";
import * as React from "react";
import { Link } from "react-router";

import { TeaBadge } from "@/components/common/tea-badge";
import { VerificationBadge } from "@/components/common/verification-badge";
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
import type { UserProfile, UserRelationship } from "@/lib/api/types";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import type { ProfileRelationshipListKind } from "./profile-relationship-list-dialog";

const roleLabels = {
  user: "社区成员",
  mod: "社区版主",
  admin: "平台管理员",
} as const;

interface ProfileSummaryProps {
  profile: UserProfile;
  relationship?: UserRelationship;
  isAuthenticated: boolean;
  isSelf: boolean;
  relationshipLoading: boolean;
  relationshipPending: boolean;
  messagePending: boolean;
  canStartConversation: boolean;
  canManageUser: boolean;
  canManageVerifications: boolean;
  confirmBlockOpen: boolean;
  onConfirmBlockOpenChange: (open: boolean) => void;
  onStartConversation: () => void;
  onToggleFollow: () => void;
  onToggleMute: () => void;
  onToggleBlock: () => void;
  onOpenRelationshipList: (kind: ProfileRelationshipListKind) => void;
  onMediaDeliveryRefresh: () => void;
}

function InlineStat({
  label,
  value,
  onClick,
}: {
  label: string;
  value: number;
  onClick?: () => void;
}) {
  const content = (
    <>
      <span className="font-semibold tabular-nums text-foreground">{formatNumber(value)}</span>
      <span className="text-muted-foreground">{label}</span>
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        onClick={onClick}
        className="inline-flex items-center gap-1 rounded-md px-0.5 py-0.5 text-[13px] outline-none transition-colors hover:text-primary focus-visible:ring-[3px] focus-visible:ring-ring/50"
      >
        {content}
      </button>
    );
  }

  return <span className="inline-flex items-center gap-1 text-[13px]">{content}</span>;
}

export function ProfileSummary({
  profile,
  relationship,
  isAuthenticated,
  isSelf,
  relationshipLoading,
  relationshipPending,
  messagePending,
  canStartConversation,
  canManageUser,
  canManageVerifications,
  confirmBlockOpen,
  onConfirmBlockOpenChange,
  onStartConversation,
  onToggleFollow,
  onToggleMute,
  onToggleBlock,
  onOpenRelationshipList,
  onMediaDeliveryRefresh,
}: ProfileSummaryProps) {
  const roleLabel = roleLabels[profile.role];
  const isBlocked = Boolean(relationship?.blockedByMe);
  const controlsPending = relationshipLoading || relationshipPending;
  const displayName = profile.displayName || profile.handle;
  const websiteLabel = profile.website?.replace(/^https?:\/\//, "").replace(/\/$/, "");
  const lastMediaRecoveryAt = React.useRef(0);
  const recoverMediaDelivery = () => {
    const now = Date.now();
    if (now - lastMediaRecoveryAt.current < 15_000) return;
    lastMediaRecoveryAt.current = now;
    onMediaDeliveryRefresh();
  };

  return (
    <>
      <Card className="overflow-hidden rounded-2xl border-border/60 shadow-none">
        <div
          className={cn(
            "relative h-[148px] overflow-hidden sm:h-[160px]",
            !profile.bannerUrl && "bg-gradient-to-br from-[#0b1220] via-[#1a2744] to-[#0f172a]",
          )}
          aria-hidden="true"
        >
          {profile.bannerUrl ? (
            <img
              src={profile.bannerUrl}
              alt=""
              aria-hidden="true"
              referrerPolicy="no-referrer"
              onError={recoverMediaDelivery}
              className="absolute inset-0 size-full object-cover"
            />
          ) : null}
        </div>

        <CardContent className="relative space-y-3 px-5 pb-5 pt-0">
          {/* Avatar + primary actions row */}
          <div className="flex items-end justify-between gap-3">
            <Avatar className="-mt-[52px] size-[104px] border-[3px] border-card bg-card shadow-sm ring-2 ring-primary/70">
              <AvatarImage
                src={profile.avatarUrl ?? undefined}
                alt={`${profile.handle} 的头像`}
                referrerPolicy="no-referrer"
                onError={recoverMediaDelivery}
              />
              <AvatarFallback className="bg-primary/10 text-3xl font-semibold text-primary">
                {profile.handle.slice(0, 1).toUpperCase()}
              </AvatarFallback>
            </Avatar>

            <div className="flex flex-wrap justify-end gap-2 pb-1 pt-3">
              {isSelf ? (
                <Button asChild variant="outline" size="sm" className="h-8 rounded-full border-border/80 bg-background px-3.5 text-xs">
                  <Link to="/settings">
                    <Settings className="size-3.5" />
                    编辑资料
                  </Link>
                </Button>
              ) : isAuthenticated ? (
                <>
                  <Button
                    type="button"
                    size="sm"
                    className="h-8 rounded-full px-4 text-xs"
                    variant={relationship?.following ? "outline" : "default"}
                    onClick={onToggleFollow}
                    disabled={controlsPending || isBlocked || !relationship?.canFollow}
                  >
                    {relationship?.following ? <UserCheck className="size-3.5" /> : <UserPlus className="size-3.5" />}
                    {relationship?.following ? "取消关注" : "关注"}
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    className="h-8 rounded-full px-3.5 text-xs"
                    onClick={onStartConversation}
                    disabled={!canStartConversation || messagePending || isBlocked}
                  >
                    <MessageSquare className="size-3.5" />
                    发私信
                  </Button>
                </>
              ) : (
                <Button asChild size="sm" className="h-8 rounded-full px-4 text-xs">
                  <Link to="/login">
                    <UserPlus className="size-3.5" />
                    登录后关注
                  </Link>
                </Button>
              )}
            </div>
          </div>

          {/* Identity block */}
          <div className="min-w-0 space-y-2.5">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="truncate text-[22px] font-bold leading-none tracking-tight">{displayName}</h1>
              <TeaBadge level={profile.trustLevel} />
              {profile.role !== "user" ? (
                <Badge variant="default" className="rounded-full px-2 py-0.5 text-[11px]">
                  {roleLabel}
                </Badge>
              ) : null}
              {relationship?.followedBy ? (
                <Badge variant="outline" className="rounded-full px-2 py-0.5 text-[11px]">
                  关注了你
                </Badge>
              ) : null}
            </div>

            <p className="text-[13px] text-muted-foreground">@{profile.handle}</p>

            {(profile.verifications ?? []).length > 0 ? (
              <div className="flex flex-wrap gap-2" aria-label="公开身份与特殊认证">
                {(profile.verifications ?? []).map((verification) => (
                  <VerificationBadge key={verification.slug} verification={verification} />
                ))}
              </div>
            ) : null}

            {profile.bio ? (
              <p className="max-w-2xl whitespace-pre-wrap text-[13px] leading-6 text-foreground/90">
                {profile.bio}
              </p>
            ) : null}

            {/* Meta row — Figma: location · join · link */}
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[12px] text-muted-foreground">
              <span className="inline-flex items-center gap-1.5">
                <MapPin className="size-3.5 shrink-0" aria-hidden="true" />
                同济大学
              </span>
              <span className="inline-flex items-center gap-1.5">
                <CalendarDays className="size-3.5 shrink-0" aria-hidden="true" />
                {formatDate(profile.createdAt)} 加入
              </span>
              {profile.website ? (
                <a
                  href={profile.website}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex max-w-[14rem] items-center gap-1.5 truncate text-primary hover:underline"
                >
                  <Link2 className="size-3.5 shrink-0" aria-hidden="true" />
                  {websiteLabel}
                </a>
              ) : null}
            </div>

            {/* Stats — Figma order: 正在关注 · 关注者 */}
            <div
              className="flex flex-wrap items-center gap-x-5 gap-y-1 pt-0.5"
              aria-label="公开社区数据"
            >
              <InlineStat
                label="正在关注"
                value={profile.followingCount}
                onClick={() => onOpenRelationshipList("following")}
              />
              <InlineStat
                label="关注者"
                value={profile.followerCount}
                onClick={() => onOpenRelationshipList("followers")}
              />
              <InlineStat label="获赞" value={profile.votesReceived} />
              <InlineStat label="发帖" value={profile.threadCount} />
            </div>
          </div>

          {!isSelf && isAuthenticated ? (
            <div className="flex flex-wrap gap-2 border-t border-border/50 pt-3">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 text-xs"
                onClick={onToggleMute}
                disabled={controlsPending || isBlocked}
              >
                {relationship?.muted ? <Volume2 className="size-3.5" /> : <VolumeX className="size-3.5" />}
                {relationship?.muted ? "取消静音" : "静音"}
              </Button>
              <Button
                type="button"
                size="sm"
                className="h-8 text-xs"
                variant={isBlocked ? "outline" : "destructive"}
                onClick={() => (isBlocked ? onToggleBlock() : onConfirmBlockOpenChange(true))}
                disabled={controlsPending}
              >
                {isBlocked ? <UserRoundCheck className="size-3.5" /> : <Ban className="size-3.5" />}
                {isBlocked ? "解除屏蔽" : "屏蔽"}
              </Button>
              {canManageUser ? (
                <Button asChild variant="secondary" size="sm" className="h-8 text-xs">
                  <Link to={`/admin?section=users&q=${encodeURIComponent(profile.handle)}`}>
                    <ShieldCheck className="size-3.5" />
                    用户管理
                  </Link>
                </Button>
              ) : null}
              {canManageVerifications ? (
                <Button asChild variant="outline" size="sm" className="h-8 text-xs">
                  <Link to={`/admin?section=verifications&account=${encodeURIComponent(profile.id)}`}>
                    <ShieldCheck className="size-3.5" />
                    管理认证
                  </Link>
                </Button>
              ) : null}
            </div>
          ) : null}

          {isAuthenticated && !isSelf && !canStartConversation && !isBlocked ? (
            <p className="text-xs text-muted-foreground">
              对方的私信策略、你的信任等级或双方关系暂不允许发起新会话。
            </p>
          ) : null}
          {isBlocked ? (
            <p className="text-xs text-muted-foreground">
              已屏蔽该用户：双方关注已移除，不能私信、回复或投票；解除后不会自动恢复关注。
            </p>
          ) : null}
        </CardContent>
      </Card>

      <Dialog open={confirmBlockOpen} onOpenChange={onConfirmBlockOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>屏蔽 {displayName}？</DialogTitle>
            <DialogDescription>
              屏蔽会立即移除双方关注，阻止私信、关注、回复和投票，并从你的信息流隐藏对方内容。解除屏蔽不会恢复原关注。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onConfirmBlockOpenChange(false)}>
              取消
            </Button>
            <Button type="button" variant="destructive" onClick={onToggleBlock} disabled={relationshipPending}>
              <Ban className="size-4" />
              确认屏蔽
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

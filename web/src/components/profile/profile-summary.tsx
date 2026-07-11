import {
  Ban,
  ExternalLink,
  Heart,
  MessageCircle,
  MessageSquare,
  Settings,
  ShieldCheck,
  Sparkles,
  UserCheck,
  UserPlus,
  UserRoundCheck,
  Volume2,
  VolumeX,
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
import type { UserProfile, UserRelationship } from "@/lib/api/types";
import { formatDate } from "@/lib/format";

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
  confirmBlockOpen: boolean;
  onConfirmBlockOpenChange: (open: boolean) => void;
  onStartConversation: () => void;
  onToggleFollow: () => void;
  onToggleMute: () => void;
  onToggleBlock: () => void;
  onOpenRelationshipList: (kind: ProfileRelationshipListKind) => void;
}

function Stat({
  icon: Icon,
  label,
  value,
  onClick,
}: {
  icon: typeof MessageSquare;
  label: string;
  value: number;
  onClick?: () => void;
}) {
  const content = (
    <>
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Icon className="size-3.5" aria-hidden="true" />
        {label}
      </div>
      <p className="mt-1 text-lg font-semibold tabular-nums">{value}</p>
    </>
  );
  return onClick ? (
    <button
      type="button"
      onClick={onClick}
      className="rounded-lg border bg-muted/25 px-3 py-2.5 text-left outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50"
    >
      {content}
    </button>
  ) : (
    <div className="rounded-lg border bg-muted/25 px-3 py-2.5">{content}</div>
  );
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
  confirmBlockOpen,
  onConfirmBlockOpenChange,
  onStartConversation,
  onToggleFollow,
  onToggleMute,
  onToggleBlock,
  onOpenRelationshipList,
}: ProfileSummaryProps) {
  const roleLabel = roleLabels[profile.role];
  const isBlocked = Boolean(relationship?.blockedByMe);
  const controlsPending = relationshipLoading || relationshipPending;

  return (
    <>
      <Card className="overflow-hidden">
        <div
          className="h-28 bg-gradient-to-r from-primary/20 via-primary/5 to-transparent bg-cover bg-center"
          style={profile.bannerUrl ? { backgroundImage: `url(${profile.bannerUrl})` } : undefined}
          aria-hidden="true"
        />
        <CardContent className="-mt-10 grid gap-5 p-5 pt-0 lg:grid-cols-[minmax(0,1fr)_25rem]">
          <div className="min-w-0">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-end">
              <Avatar className="size-24 border-4 border-card shadow-sm">
                <AvatarImage src={profile.avatarUrl ?? undefined} alt={`${profile.handle} 的头像`} />
                <AvatarFallback className="text-xl">
                  {profile.handle.slice(0, 1).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-1 pb-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="truncate text-xl font-semibold">
                    {profile.displayName || profile.handle}
                  </h2>
                  <TeaBadge level={profile.trustLevel} />
                  <Badge variant={profile.role === "user" ? "secondary" : "default"}>
                    {roleLabel}
                  </Badge>
                  {relationship?.followedBy ? <Badge variant="outline">关注了你</Badge> : null}
                </div>
                <p className="mt-1 text-sm text-muted-foreground">
                  @{profile.handle} · {formatDate(profile.createdAt)} 加入 YourTJ
                </p>
              </div>
            </div>

            {profile.bio ? <p className="mt-4 max-w-2xl whitespace-pre-wrap text-sm leading-6">{profile.bio}</p> : null}
            {profile.website ? (
              <a
                href={profile.website}
                target="_blank"
                rel="noreferrer"
                className="mt-2 inline-flex items-center gap-1 text-sm text-primary hover:underline"
              >
                <ExternalLink className="size-3.5" />
                {profile.website.replace(/^https:\/\//, "")}
              </a>
            ) : null}

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
                  <Link to="/settings"><Settings className="size-4" />编辑资料与隐私</Link>
                </Button>
              ) : isAuthenticated ? (
                <>
                  <Button
                    type="button"
                    variant={relationship?.following ? "outline" : "default"}
                    onClick={onToggleFollow}
                    disabled={controlsPending || isBlocked || !relationship?.canFollow}
                  >
                    {relationship?.following ? <UserCheck className="size-4" /> : <UserPlus className="size-4" />}
                    {relationship?.following ? "取消关注" : "关注"}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={onStartConversation}
                    disabled={!canStartConversation || messagePending || isBlocked}
                  >
                    <MessageSquare className="size-4" />发私信
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    onClick={onToggleMute}
                    disabled={controlsPending || isBlocked}
                  >
                    {relationship?.muted ? <Volume2 className="size-4" /> : <VolumeX className="size-4" />}
                    {relationship?.muted ? "取消静音" : "静音"}
                  </Button>
                  <Button
                    type="button"
                    variant={isBlocked ? "outline" : "destructive"}
                    onClick={() => isBlocked ? onToggleBlock() : onConfirmBlockOpenChange(true)}
                    disabled={controlsPending}
                  >
                    {isBlocked ? <UserRoundCheck className="size-4" /> : <Ban className="size-4" />}
                    {isBlocked ? "解除屏蔽" : "屏蔽"}
                  </Button>
                </>
              ) : (
                <Button asChild>
                  <Link to="/login"><UserPlus className="size-4" />登录后关注</Link>
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
            {isAuthenticated && !isSelf && !canStartConversation && !isBlocked ? (
              <p className="mt-2 text-xs text-muted-foreground">
                对方的私信策略、你的信任等级或双方关系暂不允许发起新会话。
              </p>
            ) : null}
            {isBlocked ? (
              <p className="mt-2 text-xs text-muted-foreground">
                已屏蔽该用户：双方关注已移除，不能私信、回复或投票；解除后不会自动恢复关注。
              </p>
            ) : null}
          </div>

          <div className="grid grid-cols-2 gap-2 self-end sm:grid-cols-5 lg:grid-cols-2" aria-label="公开社区数据">
            <Stat
              icon={UserCheck}
              label="关注者"
              value={profile.followerCount}
              onClick={() => onOpenRelationshipList("followers")}
            />
            <Stat
              icon={UserPlus}
              label="正在关注"
              value={profile.followingCount}
              onClick={() => onOpenRelationshipList("following")}
            />
            <Stat icon={MessageSquare} label="主题" value={profile.threadCount} />
            <Stat icon={MessageCircle} label="回复" value={profile.commentCount} />
            <Stat icon={Heart} label="获赞" value={profile.votesReceived} />
          </div>
        </CardContent>
      </Card>

      <Dialog open={confirmBlockOpen} onOpenChange={onConfirmBlockOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>屏蔽 {profile.displayName || profile.handle}？</DialogTitle>
            <DialogDescription>
              屏蔽会立即移除双方关注，阻止私信、关注、回复和投票，并从你的信息流隐藏对方内容。解除屏蔽不会恢复原关注。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onConfirmBlockOpenChange(false)}>
              取消
            </Button>
            <Button type="button" variant="destructive" onClick={onToggleBlock} disabled={relationshipPending}>
              <Ban className="size-4" />确认屏蔽
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

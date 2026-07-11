import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Ban, Clock3, KeyRound, Search, UserPlus, VolumeX } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { ADMIN_CAPABILITIES, hasCapability } from "@/components/admin/capabilities";
import { RecentAuthDialog } from "@/components/auth/recent-auth-dialog";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import { ApiError } from "@/lib/api/client";
import type { Account, AdminUser, AdminUserInviteInput, Sanction } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

type RoleFilter = "all" | "user" | "mod" | "admin";
type StatusFilter = "all" | "active" | "suspended" | "deleted";
type UserAction =
  | { kind: "role"; user: AdminUser; role: "user" | "mod" }
  | { kind: "silence" | "suspend" | "sessions"; user: AdminUser };

const ROLE_RANK = { user: 0, mod: 1, admin: 2 } as const;

function canManageTarget(actor: Account | null, target: AdminUser | null) {
  if (!actor?.id || !actor.role || !target || actor.id === target.id) return false;
  return ROLE_RANK[target.role] < ROLE_RANK[actor.role];
}

function localDateTimeInput(timestamp: number) {
  const date = new Date(timestamp);
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}

function InviteUserDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const queryClient = useQueryClient();
  const [email, setEmail] = React.useState("");
  const [handle, setHandle] = React.useState("");
  const [reason, setReason] = React.useState("");
  const invite = useMutation({
    mutationFn: (body: AdminUserInviteInput) => api.inviteAdminUser(body),
    onSuccess: async () => {
      toast.success("邀请已创建，账号仍需完成校园邮箱验证");
      setEmail("");
      setHandle("");
      setReason("");
      onOpenChange(false);
      await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "邀请失败"),
  });
  const isValid = email.includes("@") && handle.trim().length >= 3 && reason.trim().length >= 3;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>邀请校园用户</DialogTitle>
          <DialogDescription>创建待验证邀请，不设置密码，也不会绕过邮箱所有权证明。</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="invite-email">校园邮箱</Label>
            <Input id="invite-email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@tongji.edu.cn" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="invite-handle">公开 Handle</Label>
            <Input id="invite-handle" value={handle} onChange={(event) => setHandle(event.target.value)} minLength={3} maxLength={30} />
          </div>
          <div className="space-y-2">
            <Label>初始角色</Label>
            <div className="rounded-md border bg-muted/40 px-3 py-2 text-sm">普通用户（完成邮箱验证后可另行授予版主角色）</div>
          </div>
          <div className="space-y-2 sm:col-span-2">
            <Label htmlFor="invite-reason">邀请原因</Label>
            <Textarea id="invite-reason" value={reason} onChange={(event) => setReason(event.target.value)} maxLength={500} placeholder="该原因将进入审计记录" />
          </div>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={invite.isPending}>取消</Button>
          <Button
            type="button"
            onClick={() => invite.mutate({ email: email.trim(), handle: handle.trim(), reason: reason.trim() })}
            disabled={!isValid || invite.isPending}
          >
            {invite.isPending ? "正在创建…" : "创建邀请"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SanctionsDialog({
  user,
  capabilities,
  canMutateUser,
  onClose,
  onRecentAuthRequired,
}: {
  user: AdminUser | null;
  capabilities: Set<string>;
  canMutateUser: boolean;
  onClose: () => void;
  onRecentAuthRequired: (retry: () => void) => void;
}) {
  const queryClient = useQueryClient();
  const [revoking, setRevoking] = React.useState<Sanction | null>(null);
  const sanctions = useQuery({
    queryKey: ["admin", "users", user?.id, "sanctions"],
    queryFn: () => api.adminUserSanctions(user?.id ?? ""),
    enabled: Boolean(user?.id),
  });
  const revoke = useMutation({
    mutationFn: ({ accountId, sanctionId, reason }: { accountId: string; sanctionId: string; reason: string }) =>
      api.unsanctionAdminUser(accountId, sanctionId, reason),
    onSuccess: async (_data, variables) => {
      toast.success("制裁已撤销");
      setRevoking(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "users", variables.accountId, "sanctions"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error, variables) => {
      if (error instanceof ApiError && error.code === "RECENT_AUTH_REQUIRED") {
        setRevoking(null);
        onClose();
        onRecentAuthRequired(() => revoke.mutate(variables));
        return;
      }
      toast.error(error instanceof Error ? error.message : "撤销失败");
    },
  });

  function canRevoke(kind?: string) {
    if (!canMutateUser) return false;
    if (kind === "silence") return hasCapability(capabilities, ADMIN_CAPABILITIES.silenceUsers);
    if (kind === "suspend") return hasCapability(capabilities, ADMIN_CAPABILITIES.suspendUsers);
    return false;
  }

  return (
    <>
      <Dialog open={Boolean(user)} onOpenChange={(open) => !open && onClose()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{user?.handle} 的制裁记录</DialogTitle>
            <DialogDescription>查看历史与当前状态。只有具备对应制裁能力的工作人员可以撤销，且必须填写原因。</DialogDescription>
          </DialogHeader>
          {sanctions.isLoading ? (
            <LoadingState />
          ) : sanctions.isError ? (
            <ErrorState error={sanctions.error} onRetry={() => void sanctions.refetch()} />
          ) : (sanctions.data ?? []).length === 0 ? (
            <EmptyState title="没有制裁记录" />
          ) : (
            <div className="max-h-[55vh] space-y-2 overflow-y-auto pr-1">
              {sanctions.data?.map((sanction) => {
                const now = Math.floor(Date.now() / 1000);
                const isActive = !sanction.revokedAt
                  && (sanction.startsAt ?? 0) <= now
                  && (!sanction.endsAt || sanction.endsAt > now);
                return (
                  <Card key={sanction.id}>
                    <CardContent className="space-y-2 p-4">
                      <div className="flex flex-wrap items-center gap-2">
                        <AdminStatusBadge value={sanction.kind} />
                        <AdminStatusBadge value={sanction.revokedAt ? "已撤销" : isActive ? "生效中" : "已到期"} />
                      </div>
                      <p className="text-sm">{sanction.reason}</p>
                      <p className="text-xs text-muted-foreground">
                        {formatUnixTime(sanction.startsAt)} 至 {sanction.endsAt ? formatUnixTime(sanction.endsAt) : "未设截止时间"}
                      </p>
                      {isActive && sanction.id && canRevoke(sanction.kind) ? (
                        <Button type="button" variant="outline" size="sm" onClick={() => setRevoking(sanction)}>撤销制裁</Button>
                      ) : null}
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </DialogContent>
      </Dialog>
      <ReasonDialog
        open={Boolean(revoking)}
        onOpenChange={(open) => !open && setRevoking(null)}
        title={`撤销${revoking?.kind === "suspend" ? "封禁" : "禁言"}`}
        description="撤销不会覆盖原记录，而是追加一条新的治理审计事件。"
        confirmLabel="确认撤销"
        isPending={revoke.isPending}
        onConfirm={(reason) => revoking?.id && user?.id && revoke.mutate({ accountId: user.id, sanctionId: revoking.id, reason })}
      />
    </>
  );
}

export function UsersPanel({ capabilities, initialQuery = "" }: { capabilities: Set<string>; initialQuery?: string }) {
  const { account } = useAuth();
  const queryClient = useQueryClient();
  const [searchDraft, setSearchDraft] = React.useState(initialQuery);
  const [search, setSearch] = React.useState(initialQuery);
  const [role, setRole] = React.useState<RoleFilter>("all");
  const [status, setStatus] = React.useState<StatusFilter>("all");
  const [cursorStack, setCursorStack] = React.useState<Array<string | null>>([null]);
  const [inviteOpen, setInviteOpen] = React.useState(false);
  const [action, setAction] = React.useState<UserAction | null>(null);
  const [endsAt, setEndsAt] = React.useState("");
  const [sanctionsUser, setSanctionsUser] = React.useState<AdminUser | null>(null);
  const [recentAuthRetry, setRecentAuthRetry] = React.useState<(() => void) | null>(null);
  const cursor = cursorStack.at(-1);
  const canSearchUsers = hasCapability(capabilities, ADMIN_CAPABILITIES.searchUsers);
  const canInvite = hasCapability(capabilities, ADMIN_CAPABILITIES.inviteUsers);
  const canChangeRoles = hasCapability(capabilities, ADMIN_CAPABILITIES.changeRoles);
  const canSilence = hasCapability(capabilities, ADMIN_CAPABILITIES.silenceUsers);
  const canSuspend = hasCapability(capabilities, ADMIN_CAPABILITIES.suspendUsers);
  const sanctionTimeBounds = React.useMemo(() => {
    const now = Date.now();
    return {
      minimum: localDateTimeInput(now + 60_000),
      maximum: account?.role === "mod"
        ? localDateTimeInput(now + 30 * 24 * 60 * 60 * 1000)
        : undefined,
    };
  }, [account?.role]);

  React.useEffect(() => {
    setSearchDraft(initialQuery);
    setSearch(initialQuery);
    setCursorStack([null]);
  }, [initialQuery]);

  const users = useQuery({
    queryKey: ["admin", "users", search, role, status, cursor],
    queryFn: () =>
      api.adminUsers({
        q: search || undefined,
        role: role === "all" ? undefined : role,
        status: status === "all" ? undefined : status,
        cursor,
      }),
    enabled: canSearchUsers,
  });
  const actionMutation = useMutation({
    mutationFn: async ({ nextAction, reason }: { nextAction: UserAction; reason: string }) => {
      if (nextAction.kind === "role") {
        await api.updateAdminUserRole(nextAction.user.id, nextAction.role, reason);
        return;
      }
      if (nextAction.kind === "sessions") {
        await api.revokeAdminUserSessions(nextAction.user.id, reason);
        return;
      }
      await api.sanctionAdminUser(nextAction.user.id, nextAction.kind, {
        reason,
        endsAt: endsAt ? Math.floor(new Date(endsAt).getTime() / 1000) : null,
      });
    },
    onSuccess: async () => {
      toast.success("用户治理操作已完成");
      setAction(null);
      setEndsAt("");
      await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "overview"] });
    },
    onError: (error, variables) => {
      if (error instanceof ApiError && error.code === "RECENT_AUTH_REQUIRED") {
        setAction(null);
        setEndsAt("");
        setRecentAuthRetry(() => () => actionMutation.mutate(variables));
        return;
      }
      toast.error(error instanceof Error ? error.message : "操作失败");
    },
  });

  const actionTitle = action?.kind === "role"
    ? `修改 ${action.user.handle} 的角色`
    : action?.kind === "silence"
      ? `禁言 ${action.user.handle}`
      : action?.kind === "suspend"
        ? `封禁 ${action.user.handle}`
        : action?.kind === "sessions"
          ? `撤销 ${action.user.handle} 的全部会话`
          : "用户治理操作";
  const requiresEnd = action?.kind === "silence";

  function resetCursor() {
    setCursorStack([null]);
  }

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="用户治理"
        description="搜索公开账号资料，按权限执行邀请、角色变更、禁言、封禁和会话撤销。所有敏感操作均由后端再次校验角色层级。"
        actions={canInvite ? (
          <Button type="button" size="sm" onClick={() => setInviteOpen(true)}>
            <UserPlus className="size-4" />邀请用户
          </Button>
        ) : undefined}
      />

      {canSearchUsers ? <Card>
        <CardContent className="grid gap-3 p-4 md:grid-cols-[minmax(0,1fr)_10rem_10rem_auto] md:items-end">
          <form
            className="space-y-2"
            onSubmit={(event) => {
              event.preventDefault();
              setSearch(searchDraft.trim());
              resetCursor();
            }}
          >
            <Label htmlFor="admin-user-search">搜索</Label>
            <div className="flex gap-2">
              <Input id="admin-user-search" value={searchDraft} onChange={(event) => setSearchDraft(event.target.value)} placeholder="Handle 或账号 ID" />
              <Button type="submit" variant="outline" size="icon" aria-label="搜索用户"><Search className="size-4" /></Button>
            </div>
          </form>
          <div className="space-y-2">
            <Label htmlFor="admin-role-filter">角色</Label>
            <Select value={role} onValueChange={(value) => { setRole(value as RoleFilter); resetCursor(); }}>
              <SelectTrigger id="admin-role-filter"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部角色</SelectItem>
                <SelectItem value="user">用户</SelectItem>
                <SelectItem value="mod">版主</SelectItem>
                <SelectItem value="admin">管理员</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="admin-status-filter">状态</Label>
            <Select value={status} onValueChange={(value) => { setStatus(value as StatusFilter); resetCursor(); }}>
              <SelectTrigger id="admin-status-filter"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部状态</SelectItem>
                <SelectItem value="active">正常</SelectItem>
                <SelectItem value="suspended">已封禁</SelectItem>
                <SelectItem value="deleted">已删除</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Button type="button" variant="ghost" onClick={() => { setSearchDraft(""); setSearch(""); setRole("all"); setStatus("all"); resetCursor(); }}>
            清除筛选
          </Button>
        </CardContent>
      </Card> : (
        <EmptyState
          title="没有用户目录读取权限"
          description="仍可使用已授权的独立操作；搜索、筛选和制裁历史需要 users.search 能力。"
        />
      )}

      {!canSearchUsers ? null : users.isLoading ? (
        <LoadingState label="加载用户目录" />
      ) : users.isError ? (
        <ErrorState title="用户目录加载失败" error={users.error} onRetry={() => void users.refetch()} />
      ) : (users.data?.items ?? []).length === 0 ? (
        <EmptyState title="没有符合条件的用户" description="调整搜索词、角色或状态筛选后重试。" />
      ) : (
        <div className="space-y-3">
          {users.data?.items?.map((user) => {
            const isSelf = user.id === account?.id;
            const canActOnUser = canManageTarget(account, user);
            return (
              <Card key={user.id} className="rounded-xl">
                <CardContent className="flex flex-col gap-4 p-4 xl:flex-row xl:items-center">
                  <div className="flex min-w-0 flex-1 items-center gap-3">
                    <Avatar className="size-10">
                      <AvatarImage src={user.avatarUrl ?? undefined} />
                      <AvatarFallback>{user.handle.slice(0, 1).toUpperCase()}</AvatarFallback>
                    </Avatar>
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="truncate font-medium">{user.handle}</p>
                        <AdminStatusBadge value={user.role} />
                        <AdminStatusBadge value={user.status} />
                        {isSelf ? (
                          <span className="rounded-full border bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">当前账号</span>
                        ) : null}
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        ID {user.id} · TL {user.trustLevel} · 最近活跃 {user.lastActiveAt ? formatUnixTime(user.lastActiveAt) : "暂无"}
                      </p>
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button type="button" variant="outline" size="sm" onClick={() => setSanctionsUser(user)}>
                      <Clock3 className="size-4" />制裁记录
                    </Button>
                    {canActOnUser && canChangeRoles ? (
                      <Select value={user.role} onValueChange={(value) => setAction({ kind: "role", user, role: value as "user" | "mod" })}>
                        <SelectTrigger className="h-8 w-28 text-xs" aria-label={`修改 ${user.handle} 的角色`}><SelectValue /></SelectTrigger>
                        <SelectContent>
                          <SelectItem value="user">用户</SelectItem>
                          <SelectItem value="mod">版主</SelectItem>
                        </SelectContent>
                      </Select>
                    ) : null}
                    {canActOnUser && canSilence && user.status === "active" ? (
                      <Button type="button" variant="outline" size="sm" onClick={() => setAction({ kind: "silence", user })}>
                        <VolumeX className="size-4" />禁言
                      </Button>
                    ) : null}
                    {canActOnUser && canSuspend && user.status === "active" ? (
                      <Button type="button" variant="destructive" size="sm" onClick={() => setAction({ kind: "suspend", user })}>
                        <Ban className="size-4" />封禁
                      </Button>
                    ) : null}
                    {canActOnUser && canSuspend ? (
                      <Button type="button" variant="outline" size="sm" onClick={() => setAction({ kind: "sessions", user })}>
                        <KeyRound className="size-4" />撤销会话
                      </Button>
                    ) : null}
                  </div>
                </CardContent>
              </Card>
            );
          })}
          <PaginationControls
            hasPrevious={cursorStack.length > 1}
            hasMore={Boolean(users.data?.hasMore && users.data.nextCursor)}
            onPrevious={() => setCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
            onNext={() => users.data?.nextCursor && setCursorStack((items) => [...items, users.data?.nextCursor ?? null])}
          />
        </div>
      )}

      <InviteUserDialog open={inviteOpen} onOpenChange={setInviteOpen} />
      <SanctionsDialog
        user={sanctionsUser}
        capabilities={capabilities}
        canMutateUser={canManageTarget(account, sanctionsUser)}
        onClose={() => setSanctionsUser(null)}
        onRecentAuthRequired={(retry) => setRecentAuthRetry(() => retry)}
      />
      <ReasonDialog
        open={Boolean(action)}
        onOpenChange={(open) => { if (!open) { setAction(null); setEndsAt(""); } }}
        title={actionTitle}
        description="提交后会立即生效，并写入治理审计记录。服务端会阻止自操作、越级操作和最后管理员变更。"
        confirmLabel={action?.kind === "suspend" ? "确认封禁" : "确认执行"}
        destructive={action?.kind === "suspend" || action?.kind === "sessions"}
        isPending={actionMutation.isPending}
        confirmDisabled={requiresEnd && !endsAt}
        onConfirm={(reason) => action && actionMutation.mutate({ nextAction: action, reason })}
      >
        {action?.kind === "role" ? (
          <div className="rounded-lg border bg-muted/50 p-3 text-sm">
            新角色：<span className="font-medium"><AdminStatusBadge value={action.role} /></span>
          </div>
        ) : null}
        {action?.kind === "silence" || action?.kind === "suspend" ? (
          <div className="space-y-2">
            <Label htmlFor="admin-sanction-end">截止时间{action.kind === "silence" ? "（必填）" : "（可选）"}</Label>
            <Input
              id="admin-sanction-end"
              type="datetime-local"
              value={endsAt}
              min={sanctionTimeBounds.minimum}
              max={sanctionTimeBounds.maximum}
              onChange={(event) => setEndsAt(event.target.value)}
            />
            {action.kind === "silence" && account?.role === "mod" ? (
              <p className="text-xs text-muted-foreground">版主禁言必须设置截止时间，且最长 30 天。</p>
            ) : null}
          </div>
        ) : null}
      </ReasonDialog>
      <RecentAuthDialog
        open={recentAuthRetry !== null}
        onOpenChange={(open) => { if (!open) setRecentAuthRetry(null); }}
        onVerified={() => {
          const retry = recentAuthRetry;
          setRecentAuthRetry(null);
          retry?.();
        }}
      />
    </div>
  );
}

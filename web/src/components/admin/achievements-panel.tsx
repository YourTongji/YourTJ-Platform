import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Award,
  BookOpenCheck,
  History,
  MessageCircleHeart,
  Pencil,
  Plus,
  RotateCcw,
  Star,
  UserRoundCheck,
} from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api/endpoints";
import type {
  Achievement,
  AchievementGrant,
  AchievementIcon,
  AchievementStatus,
} from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

const iconOptions: Array<[AchievementIcon, string]> = [
  ["award", "奖章"],
  ["book-open-check", "学习贡献"],
  ["message-circle-heart", "社区互动"],
  ["star", "精选贡献"],
];

const iconComponents = {
  award: Award,
  "book-open-check": BookOpenCheck,
  "message-circle-heart": MessageCircleHeart,
  star: Star,
} satisfies Record<AchievementIcon, React.ComponentType<{ className?: string }>>;

function AchievementMark({ icon }: { icon: AchievementIcon }) {
  const Icon = iconComponents[icon];
  return (
    <span className="grid size-9 shrink-0 place-items-center rounded-full bg-primary/10 text-primary">
      <Icon className="size-4" aria-hidden="true" />
    </span>
  );
}

function SelectField({
  id,
  label,
  value,
  onChange,
  options,
  disabled = false,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<[string, string]>;
  disabled?: boolean;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <select
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
        className="h-9 w-full rounded-md border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-50"
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>{optionLabel}</option>
        ))}
      </select>
    </div>
  );
}

type DefinitionForm = {
  slug: string;
  name: string;
  description: string;
  icon: AchievementIcon;
  status: AchievementStatus;
  mintAmount: string;
  reason: string;
};

const emptyDefinition: DefinitionForm = {
  slug: "",
  name: "",
  description: "",
  icon: "award",
  status: "active",
  mintAmount: "0",
  reason: "",
};

export function AchievementsPanel({ initialAccountId = "" }: { initialAccountId?: string }) {
  const queryClient = useQueryClient();
  const [definitionCursorStack, setDefinitionCursorStack] = React.useState<Array<string | null>>([
    null,
  ]);
  const definitionCursor = definitionCursorStack.at(-1);
  const [editing, setEditing] = React.useState<Achievement | null>(null);
  const [definitionForm, setDefinitionForm] = React.useState<DefinitionForm>(emptyDefinition);
  const [retiring, setRetiring] = React.useState<Achievement | null>(null);
  const [accountId, setAccountId] = React.useState(initialAccountId);
  const normalizedAccountId = /^\d+$/.test(accountId.trim()) ? accountId.trim() : "";
  const [achievementId, setAchievementId] = React.useState("");
  const [grantReason, setGrantReason] = React.useState("");
  const [revoking, setRevoking] = React.useState<AchievementGrant | null>(null);
  const [grantCursorStack, setGrantCursorStack] = React.useState<Array<string | null>>([null]);
  const [eventCursorStack, setEventCursorStack] = React.useState<Array<string | null>>([null]);
  const grantCursor = grantCursorStack.at(-1);
  const eventCursor = eventCursorStack.at(-1);

  const definitions = useQuery({
    queryKey: ["admin", "achievements", definitionCursor],
    queryFn: () => api.adminAchievements(definitionCursor),
  });
  const definitionItems = React.useMemo(
    () => definitions.data?.items ?? [],
    [definitions.data?.items],
  );
  const activeDefinitions = definitionItems.filter((definition) => definition.status === "active");

  React.useEffect(() => {
    if (
      activeDefinitions[0]
      && !activeDefinitions.some((definition) => definition.id === achievementId)
    ) {
      setAchievementId(activeDefinitions[0].id);
    }
  }, [achievementId, activeDefinitions]);

  React.useEffect(() => {
    setGrantCursorStack([null]);
    setEventCursorStack([null]);
  }, [normalizedAccountId]);

  const grants = useQuery({
    queryKey: ["admin", "user-achievements", normalizedAccountId, grantCursor],
    queryFn: () => api.adminUserAchievements(normalizedAccountId, grantCursor),
    enabled: Boolean(normalizedAccountId),
  });
  const events = useQuery({
    queryKey: ["admin", "user-achievement-events", normalizedAccountId, eventCursor],
    queryFn: () => api.adminUserAchievementEvents(normalizedAccountId, eventCursor),
    enabled: Boolean(normalizedAccountId),
  });

  const resetDefinition = () => {
    setEditing(null);
    setDefinitionForm(emptyDefinition);
  };

  const editDefinition = (definition: Achievement) => {
    setEditing(definition);
    setDefinitionForm({
      slug: definition.slug,
      name: definition.name,
      description: definition.description ?? "",
      icon: definition.icon,
      status: definition.status,
      mintAmount: String(definition.mintAmount),
      reason: "",
    });
  };

  const saveDefinition = useMutation({
    mutationFn: () => {
      const mintAmount = Number.parseInt(definitionForm.mintAmount, 10);
      if (editing) {
        return api.updateAdminAchievement(editing.id, {
          expectedVersion: editing.version,
          name: definitionForm.name.trim(),
          description: definitionForm.description.trim() || null,
          icon: definitionForm.icon,
          status: definitionForm.status,
          mintAmount,
          reason: definitionForm.reason.trim(),
        });
      }
      return api.createAdminAchievement({
        slug: definitionForm.slug.trim(),
        name: definitionForm.name.trim(),
        description: definitionForm.description.trim() || null,
        icon: definitionForm.icon,
        mintAmount,
        reason: definitionForm.reason.trim(),
      });
    },
    onSuccess: async () => {
      toast.success(editing ? "成就定义已更新" : "成就定义已创建");
      resetDefinition();
      setDefinitionCursorStack([null]);
      await queryClient.invalidateQueries({ queryKey: ["admin", "achievements"] });
    },
    onError: async (error) => {
      toast.error(
        error instanceof Error ? error.message : "成就定义保存失败，请刷新后重试",
      );
      await queryClient.invalidateQueries({ queryKey: ["admin", "achievements"] });
    },
  });

  const retireDefinition = useMutation({
    mutationFn: (reason: string) => {
      if (!retiring) throw new Error("没有待停用的成就定义");
      return api.updateAdminAchievement(retiring.id, {
        expectedVersion: retiring.version,
        name: retiring.name,
        description: retiring.description,
        icon: retiring.icon,
        status: "retired",
        mintAmount: retiring.mintAmount,
        reason,
      });
    },
    onSuccess: async () => {
      toast.success("成就定义已停用，历史授予仍会保留");
      setRetiring(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "achievements"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "停用失败"),
  });

  const grantAchievement = useMutation({
    mutationFn: () => api.grantAdminUserAchievement(normalizedAccountId, {
      achievementId,
      reason: grantReason.trim(),
    }),
    onSuccess: async () => {
      toast.success("成就已人工授予；本次操作不会发放积分");
      setGrantReason("");
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["admin", "user-achievements", normalizedAccountId],
        }),
        queryClient.invalidateQueries({
          queryKey: ["admin", "user-achievement-events", normalizedAccountId],
        }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "成就授予失败"),
  });

  const revokeAchievement = useMutation({
    mutationFn: (reason: string) => {
      if (!revoking) throw new Error("没有待撤销的成就");
      return api.revokeAdminUserAchievement(
        normalizedAccountId,
        revoking.achievementId,
        reason,
      );
    },
    onSuccess: async () => {
      toast.success("成就已撤销；历史积分与账本不会被改写");
      setRevoking(null);
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["admin", "user-achievements", normalizedAccountId],
        }),
        queryClient.invalidateQueries({
          queryKey: ["admin", "user-achievement-events", normalizedAccountId],
        }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "成就撤销失败"),
  });

  const mintAmount = Number.parseInt(definitionForm.mintAmount, 10);
  const definitionReady = definitionForm.name.trim().length > 0
    && (editing !== null || definitionForm.slug.trim().length > 0)
    && Number.isInteger(mintAmount)
    && mintAmount >= 0
    && definitionForm.reason.trim().length >= 3;
  const grantReady = Boolean(
    normalizedAccountId && achievementId && grantReason.trim().length >= 3,
  );

  return (
    <div className="space-y-6">
      <AdminSectionHeader
        title="贡献成就"
        description="成就是贡献里程碑，不是管理员角色或身份认证。自动规则可以按定义发放贡献积分；人工授予只展示荣誉，永远不会凭空增加积分。"
        actions={editing ? (
          <Button type="button" variant="outline" onClick={resetDefinition}>取消编辑</Button>
        ) : undefined}
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(20rem,0.85fr)_minmax(0,1.35fr)]">
        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              {editing ? <Pencil className="size-4 text-primary" /> : <Plus className="size-4 text-primary" />}
              {editing ? "编辑成就定义" : "新建成就定义"}
            </CardTitle>
            <CardDescription>
              图标使用受控设计令牌；积分数只在自动贡献规则首次命中时生效。
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form
              className="space-y-4"
              onSubmit={(event) => {
                event.preventDefault();
                if (definitionReady) saveDefinition.mutate();
              }}
            >
              <div className="space-y-2">
                <Label htmlFor="achievement-slug">Slug</Label>
                <Input
                  id="achievement-slug"
                  value={definitionForm.slug}
                  onChange={(event) => setDefinitionForm((form) => ({
                    ...form,
                    slug: event.target.value,
                  }))}
                  placeholder="community-helper"
                  disabled={Boolean(editing)}
                  maxLength={64}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="achievement-name">展示名称</Label>
                <Input
                  id="achievement-name"
                  value={definitionForm.name}
                  onChange={(event) => setDefinitionForm((form) => ({
                    ...form,
                    name: event.target.value,
                  }))}
                  maxLength={100}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="achievement-description">说明（可选）</Label>
                <Textarea
                  id="achievement-description"
                  value={definitionForm.description}
                  onChange={(event) => setDefinitionForm((form) => ({
                    ...form,
                    description: event.target.value,
                  }))}
                  maxLength={240}
                />
              </div>
              <div className="grid gap-4 sm:grid-cols-2">
                <SelectField
                  id="achievement-icon"
                  label="图标"
                  value={definitionForm.icon}
                  onChange={(value) => setDefinitionForm((form) => ({
                    ...form,
                    icon: value as AchievementIcon,
                  }))}
                  options={iconOptions}
                />
                {editing ? (
                  <SelectField
                    id="achievement-status"
                    label="状态"
                    value={definitionForm.status}
                    onChange={(value) => setDefinitionForm((form) => ({
                      ...form,
                      status: value as AchievementStatus,
                    }))}
                    options={[["active", "启用"], ["retired", "停用"]]}
                  />
                ) : (
                  <div className="space-y-2">
                    <Label htmlFor="achievement-mint">自动规则积分</Label>
                    <Input
                      id="achievement-mint"
                      type="number"
                      min={0}
                      max={100000}
                      value={definitionForm.mintAmount}
                      onChange={(event) => setDefinitionForm((form) => ({
                        ...form,
                        mintAmount: event.target.value,
                      }))}
                    />
                  </div>
                )}
              </div>
              {editing ? (
                <div className="space-y-2">
                  <Label htmlFor="achievement-mint">自动规则积分</Label>
                  <Input
                    id="achievement-mint"
                    type="number"
                    min={0}
                    max={100000}
                    value={definitionForm.mintAmount}
                    onChange={(event) => setDefinitionForm((form) => ({
                      ...form,
                      mintAmount: event.target.value,
                    }))}
                  />
                </div>
              ) : null}
              <div className="space-y-2">
                <Label htmlFor="achievement-definition-reason">操作原因</Label>
                <Textarea
                  id="achievement-definition-reason"
                  value={definitionForm.reason}
                  onChange={(event) => setDefinitionForm((form) => ({
                    ...form,
                    reason: event.target.value,
                  }))}
                  placeholder="原因将写入操作记录"
                  maxLength={500}
                />
              </div>
              <Button type="submit" disabled={!definitionReady || saveDefinition.isPending}>
                {saveDefinition.isPending ? "正在保存…" : editing ? "保存定义" : "创建成就"}
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><Award className="size-4 text-primary" />成就目录</CardTitle>
            <CardDescription>停用会阻止后续自动或人工授予，但不会抹掉用户的历史记录。</CardDescription>
          </CardHeader>
          <CardContent>
            {definitions.isLoading ? (
              <LoadingState label="加载成就目录" />
            ) : definitions.isError ? (
              <ErrorState error={definitions.error} onRetry={() => void definitions.refetch()} />
            ) : definitionItems.length === 0 ? (
              <EmptyState title="尚无成就定义" />
            ) : (
              <div className="space-y-3">
                {definitionItems.map((definition) => (
                  <article key={definition.id} className="rounded-xl border p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="flex min-w-0 gap-3">
                        <AchievementMark icon={definition.icon} />
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <h3 className="font-medium">{definition.name}</h3>
                            <AdminStatusBadge value={definition.status} />
                            <Badge variant="outline">自动 +{definition.mintAmount} 积分</Badge>
                          </div>
                          <p className="mt-1 text-xs text-muted-foreground">{definition.slug} · v{definition.version}</p>
                          {definition.description ? (
                            <p className="mt-2 text-sm leading-6 text-muted-foreground">{definition.description}</p>
                          ) : null}
                        </div>
                      </div>
                      <div className="flex gap-2">
                        <Button type="button" variant="outline" size="sm" onClick={() => editDefinition(definition)}>
                          <Pencil className="size-3.5" />编辑
                        </Button>
                        {definition.status === "active" ? (
                          <Button type="button" variant="outline" size="sm" onClick={() => setRetiring(definition)}>
                            停用
                          </Button>
                        ) : null}
                      </div>
                    </div>
                  </article>
                ))}
                <PaginationControls
                  hasPrevious={definitionCursorStack.length > 1}
                  hasMore={Boolean(definitions.data?.hasMore && definitions.data.nextCursor)}
                  onPrevious={() => setDefinitionCursorStack((stack) => stack.slice(0, -1))}
                  onNext={() => {
                    if (definitions.data?.nextCursor) {
                      setDefinitionCursorStack((stack) => [
                        ...stack,
                        definitions.data?.nextCursor ?? null,
                      ]);
                    }
                  }}
                />
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <Card className="rounded-xl">
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><UserRoundCheck className="size-4 text-primary" />用户成就操作</CardTitle>
          <CardDescription>只允许操作层级更低的账号。人工授予与撤销均保留历史，且都不会改写积分账本。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="grid gap-4 lg:grid-cols-[12rem_minmax(14rem,1fr)_minmax(16rem,1.2fr)_auto] lg:items-end">
            <div className="space-y-2">
              <Label htmlFor="achievement-account-id">账号 ID</Label>
              <Input
                id="achievement-account-id"
                inputMode="numeric"
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
                placeholder="42"
              />
            </div>
            <SelectField
              id="achievement-grant-definition"
              label="成就"
              value={achievementId}
              onChange={setAchievementId}
              options={activeDefinitions.map((definition) => [definition.id, definition.name])}
              disabled={activeDefinitions.length === 0}
            />
            <div className="space-y-2">
              <Label htmlFor="achievement-grant-reason">授予原因</Label>
              <Input
                id="achievement-grant-reason"
                value={grantReason}
                onChange={(event) => setGrantReason(event.target.value)}
                maxLength={500}
                placeholder="人工确认的贡献依据"
              />
            </div>
            <Button type="button" onClick={() => grantAchievement.mutate()} disabled={!grantReady || grantAchievement.isPending}>
              <Plus className="size-4" />{grantAchievement.isPending ? "授予中…" : "授予成就"}
            </Button>
          </div>

          {!accountId.trim() ? (
            <EmptyState title="输入账号 ID 以查看成就" description="可以从用户管理页复制稳定账号 ID。" />
          ) : !normalizedAccountId ? (
            <EmptyState title="账号 ID 格式不正确" description="账号 ID 只能包含数字。" />
          ) : grants.isLoading || events.isLoading ? (
            <LoadingState label="加载用户成就与历史" />
          ) : grants.isError || events.isError ? (
            <ErrorState
              title="用户成就加载失败"
              error={grants.error ?? events.error}
              onRetry={() => {
                void grants.refetch();
                void events.refetch();
              }}
            />
          ) : (
            <div className="grid gap-4 xl:grid-cols-2">
              <section aria-labelledby="achievement-grants-title" className="rounded-xl border p-4">
                <h3 id="achievement-grants-title" className="font-medium">授予记录</h3>
                {(grants.data?.items ?? []).length === 0 ? (
                  <EmptyState title="该账号尚无成就" />
                ) : (
                  <div className="mt-3 space-y-3">
                    {grants.data?.items?.map((grant) => (
                      <article key={grant.achievementId} className="flex flex-wrap items-start justify-between gap-3 rounded-lg bg-muted/45 p-3">
                        <div className="flex gap-3">
                          <AchievementMark icon={grant.icon} />
                          <div>
                            <div className="flex flex-wrap items-center gap-2">
                              <p className="text-sm font-medium">{grant.name}</p>
                              <AdminStatusBadge value={grant.status} />
                            </div>
                            <p className="mt-1 text-xs text-muted-foreground">{formatUnixTime(grant.awardedAt)} · {grant.awardReason ?? "历史记录未填写原因"}</p>
                            {grant.revokeReason ? <p className="mt-1 text-xs text-destructive">撤销：{grant.revokeReason}</p> : null}
                          </div>
                        </div>
                        {grant.status === "active" ? (
                          <Button type="button" variant="outline" size="sm" onClick={() => setRevoking(grant)}>撤销</Button>
                        ) : (
                          <Badge variant="outline"><RotateCcw className="size-3" />可重新授予</Badge>
                        )}
                      </article>
                    ))}
                    <PaginationControls
                      hasPrevious={grantCursorStack.length > 1}
                      hasMore={Boolean(grants.data?.hasMore && grants.data.nextCursor)}
                      onPrevious={() => setGrantCursorStack((stack) => stack.slice(0, -1))}
                      onNext={() => {
                        if (grants.data?.nextCursor) {
                          setGrantCursorStack((stack) => [
                            ...stack,
                            grants.data?.nextCursor ?? null,
                          ]);
                        }
                      }}
                    />
                  </div>
                )}
              </section>

              <section aria-labelledby="achievement-events-title" className="rounded-xl border p-4">
                <h3 id="achievement-events-title" className="flex items-center gap-2 font-medium"><History className="size-4 text-primary" />不可变事件历史</h3>
                {(events.data?.items ?? []).length === 0 ? (
                  <EmptyState title="尚无成就事件" />
                ) : (
                  <ol className="mt-3 space-y-3">
                    {events.data?.items?.map((event) => (
                      <li key={event.id} className="rounded-lg border-l-2 border-primary/30 pl-3 text-sm">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="font-medium">{event.name}</span>
                          <AdminStatusBadge value={event.action === "awarded" ? "active" : "revoked"} />
                          <Badge variant="outline">{event.source === "automatic" ? "自动规则" : "人工操作"}</Badge>
                        </div>
                        <p className="mt-1 leading-5 text-muted-foreground">{event.reason}</p>
                        <p className="mt-1 text-xs text-muted-foreground">{formatUnixTime(event.createdAt)} · 操作者 {event.actorId ?? "系统"}</p>
                      </li>
                    ))}
                  </ol>
                )}
                <PaginationControls
                  hasPrevious={eventCursorStack.length > 1}
                  hasMore={Boolean(events.data?.hasMore && events.data.nextCursor)}
                  onPrevious={() => setEventCursorStack((stack) => stack.slice(0, -1))}
                  onNext={() => {
                    if (events.data?.nextCursor) {
                      setEventCursorStack((stack) => [...stack, events.data?.nextCursor ?? null]);
                    }
                  }}
                />
              </section>
            </div>
          )}
        </CardContent>
      </Card>

      <ReasonDialog
        open={Boolean(retiring)}
        onOpenChange={(open) => {
          if (!open) setRetiring(null);
        }}
        title="停用成就定义"
        description="后续自动规则与人工操作都不能再授予它；现有用户的历史成就不会被删除。"
        confirmLabel="确认停用"
        destructive
        isPending={retireDefinition.isPending}
        onConfirm={(reason) => retireDefinition.mutate(reason)}
      />
      <ReasonDialog
        open={Boolean(revoking)}
        onOpenChange={(open) => {
          if (!open) setRevoking(null);
        }}
        title="撤销用户成就"
        description="公开主页将不再展示该成就。历史事件与此前因真实贡献发放的积分不会被删除或冲销。"
        confirmLabel="确认撤销"
        destructive
        isPending={revokeAchievement.isPending}
        onConfirm={(reason) => revokeAchievement.mutate(reason)}
      />
    </div>
  );
}

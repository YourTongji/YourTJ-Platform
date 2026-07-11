import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BadgeCheck, History, KeyRound, Plus, ShieldX } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import {
  AdminSectionHeader,
  AdminStatusBadge,
  PaginationControls,
  ReasonDialog,
} from "@/components/admin/admin-primitives";
import { VerificationBadge } from "@/components/common/verification-badge";
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
  VerificationBadgeVariant,
  VerificationCategory,
  VerificationGrant,
  VerificationIcon,
  VerificationType,
} from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";

function unixDateTime(value: string) {
  if (!value) return null;
  const timestamp = new Date(value).getTime();
  return Number.isFinite(timestamp) ? Math.floor(timestamp / 1000) : null;
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

function ToggleField({
  id,
  label,
  description,
  checked,
  onChange,
  disabled = false,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label htmlFor={id} className="flex items-start gap-3 rounded-lg border p-3">
      <input
        id={id}
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        disabled={disabled}
        className="mt-1 size-4 accent-primary"
      />
      <span>
        <span className="block text-sm font-medium">{label}</span>
        <span className="mt-1 block text-xs leading-5 text-muted-foreground">{description}</span>
      </span>
    </label>
  );
}

export function VerificationsPanel({ initialAccountId = "" }: { initialAccountId?: string }) {
  const queryClient = useQueryClient();
  const [slug, setSlug] = React.useState("");
  const [category, setCategory] = React.useState<VerificationCategory>("identity");
  const [label, setLabel] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [icon, setIcon] = React.useState<VerificationIcon>("badge-check");
  const [badgeVariant, setBadgeVariant] = React.useState<VerificationBadgeVariant>("default");
  const [allowsPublicDisplay, setAllowsPublicDisplay] = React.useState(false);
  const [definitionReason, setDefinitionReason] = React.useState("");
  const [accountId, setAccountId] = React.useState(initialAccountId);
  const [verificationTypeId, setVerificationTypeId] = React.useState("");
  const [displayOnProfile, setDisplayOnProfile] = React.useState(false);
  const [expiresAt, setExpiresAt] = React.useState("");
  const [evidenceReference, setEvidenceReference] = React.useState("");
  const [grantReason, setGrantReason] = React.useState("");
  const [revoking, setRevoking] = React.useState<VerificationGrant | null>(null);
  const [typeCursorStack, setTypeCursorStack] = React.useState<Array<string | null>>([null]);
  const [grantCursorStack, setGrantCursorStack] = React.useState<Array<string | null>>([null]);
  const typeCursor = typeCursorStack.at(-1);
  const grantCursor = grantCursorStack.at(-1);

  const types = useQuery({
    queryKey: ["admin", "verification-types", typeCursor],
    queryFn: () => api.adminVerificationTypes(typeCursor),
  });
  const typeItems = React.useMemo(() => types.data?.items ?? [], [types.data?.items]);
  React.useEffect(() => {
    if (typeItems[0] && !typeItems.some((item) => item.id === verificationTypeId)) {
      setVerificationTypeId(typeItems[0].id);
    }
  }, [typeItems, verificationTypeId]);
  const selectedType = typeItems.find((item) => item.id === verificationTypeId);
  React.useEffect(() => {
    if (!selectedType?.allowsPublicDisplay) {
      setDisplayOnProfile(false);
    }
  }, [selectedType]);

  const normalizedAccountId = /^\d+$/.test(accountId.trim()) ? accountId.trim() : "";
  React.useEffect(() => {
    setGrantCursorStack([null]);
  }, [normalizedAccountId]);
  const grants = useQuery({
    queryKey: ["admin", "user-verifications", normalizedAccountId, grantCursor],
    queryFn: () => api.adminUserVerifications(normalizedAccountId, grantCursor),
    enabled: Boolean(normalizedAccountId),
  });

  const createType = useMutation({
    mutationFn: () => api.createAdminVerificationType({
      slug: slug.trim(),
      category,
      label: label.trim(),
      description: description.trim() || null,
      icon,
      badgeVariant,
      allowsPublicDisplay,
      reason: definitionReason.trim(),
    }),
    onSuccess: async (created) => {
      toast.success("认证类型已创建");
      setSlug("");
      setLabel("");
      setDescription("");
      setAllowsPublicDisplay(false);
      setDefinitionReason("");
      setVerificationTypeId(created.id);
      setTypeCursorStack([null]);
      await queryClient.invalidateQueries({ queryKey: ["admin", "verification-types"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "认证类型创建失败"),
  });

  const grant = useMutation({
    mutationFn: () => api.grantAdminUserVerification(normalizedAccountId, {
      verificationTypeId,
      displayOnProfile,
      expiresAt: unixDateTime(expiresAt),
      evidenceReference: evidenceReference.trim() || null,
      reason: grantReason.trim(),
    }),
    onSuccess: async () => {
      toast.success("认证已授予");
      setExpiresAt("");
      setEvidenceReference("");
      setGrantReason("");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "user-verifications", normalizedAccountId] }),
        queryClient.invalidateQueries({ queryKey: ["profile"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "认证授予失败"),
  });

  const revoke = useMutation({
    mutationFn: ({ item, reason }: { item: VerificationGrant; reason: string }) =>
      api.revokeAdminUserVerification(item.id, reason),
    onSuccess: async () => {
      toast.success("认证已撤销，历史记录保留");
      setRevoking(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["admin", "user-verifications", normalizedAccountId] }),
        queryClient.invalidateQueries({ queryKey: ["profile"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "认证撤销失败"),
  });

  const canCreateType = slug.trim().length > 0
    && label.trim().length > 0
    && definitionReason.trim().length >= 3;
  const canGrant = Boolean(normalizedAccountId)
    && Boolean(verificationTypeId)
    && grantReason.trim().length >= 3
    && (!displayOnProfile || Boolean(selectedType?.allowsPublicDisplay));

  return (
    <div className="space-y-5">
      <AdminSectionHeader
        title="身份与特殊认证"
        description="人工认证是可到期、可撤销且有证据引用的治理凭证。贡献成就由独立规则产生，管理员角色则来自实时权限，两者都不能在这里伪造。"
      />

      <Card className="rounded-xl">
        <CardContent className="grid gap-3 p-4 sm:grid-cols-3">
          <div className="rounded-lg border bg-muted/25 p-3">
            <Badge variant="outline">成就徽章</Badge>
            <p className="mt-2 text-xs leading-5 text-muted-foreground">由发帖、贡献等规则授予，可关联积分；不代表身份核验。</p>
          </div>
          <div className="rounded-lg border bg-muted/25 p-3">
            <Badge variant="secondary">身份 / 特殊认证</Badge>
            <p className="mt-2 text-xs leading-5 text-muted-foreground">由授权管理员基于私有证据签发，可设有效期并撤销。</p>
          </div>
          <div className="rounded-lg border bg-muted/25 p-3">
            <Badge>角色标识</Badge>
            <p className="mt-2 text-xs leading-5 text-muted-foreground">版主与管理员标识来自当前角色，不复制为永久徽章。</p>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-5 xl:grid-cols-2">
        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><Plus className="size-4 text-primary" />创建认证类型</CardTitle>
            <CardDescription>名称和展示只能使用纯文本、受控图标 token 与现有 Badge variant。</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="verification-slug">Slug</Label>
              <Input id="verification-slug" value={slug} onChange={(event) => setSlug(event.target.value)} maxLength={64} placeholder="official-organization" />
            </div>
            <SelectField id="verification-category" label="认证类别" value={category} onChange={(value) => setCategory(value as VerificationCategory)} options={[["identity", "身份认证"], ["special", "特殊认证"]]} />
            <div className="space-y-2 sm:col-span-2">
              <Label htmlFor="verification-label">公开标签</Label>
              <Input id="verification-label" value={label} onChange={(event) => setLabel(event.target.value)} maxLength={80} placeholder="官方学生组织" />
            </div>
            <div className="space-y-2 sm:col-span-2">
              <Label htmlFor="verification-description">公开说明（可选）</Label>
              <Textarea id="verification-description" value={description} onChange={(event) => setDescription(event.target.value)} maxLength={240} />
            </div>
            <SelectField id="verification-icon" label="受控图标" value={icon} onChange={(value) => setIcon(value as VerificationIcon)} options={[["badge-check", "核验标记"], ["building-2", "组织"], ["shield-check", "安全核验"], ["sparkles", "特殊标识"]]} />
            <SelectField id="verification-variant" label="语义样式" value={badgeVariant} onChange={(value) => setBadgeVariant(value as VerificationBadgeVariant)} options={[["default", "主色"], ["secondary", "次级"], ["outline", "描边"]]} />
            <div className="sm:col-span-2">
              <ToggleField id="verification-allows-public" label="允许公开展示" description="关闭时，任何授予都只能保留在治理记录中。安全默认是关闭。" checked={allowsPublicDisplay} onChange={setAllowsPublicDisplay} />
            </div>
            <div className="space-y-2 sm:col-span-2">
              <Label htmlFor="verification-definition-reason">创建原因</Label>
              <Textarea id="verification-definition-reason" value={definitionReason} onChange={(event) => setDefinitionReason(event.target.value)} maxLength={500} placeholder="说明认证对象、证据标准与展示依据" />
              <p className="text-xs text-muted-foreground">不要填写邮箱、口令或证据正文。</p>
            </div>
            <Button type="button" className="sm:col-span-2" onClick={() => createType.mutate()} disabled={!canCreateType || createType.isPending}>
              <Plus className="size-4" />{createType.isPending ? "正在创建…" : "创建认证类型"}
            </Button>
          </CardContent>
        </Card>

        <Card className="rounded-xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><KeyRound className="size-4 text-primary" />授予认证</CardTitle>
            <CardDescription>只能处理低于当前操作者角色的账号。证据字段只接受内部 opaque reference，不接受 URL 或正文。</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2 sm:col-span-2">
              <Label htmlFor="verification-account">用户 ID</Label>
              <Input id="verification-account" inputMode="numeric" value={accountId} onChange={(event) => setAccountId(event.target.value)} placeholder="先在用户管理中查找精确账号 ID" />
            </div>
            <div className="sm:col-span-2">
              <SelectField id="verification-type" label="认证类型" value={verificationTypeId} onChange={setVerificationTypeId} disabled={typeItems.length === 0} options={typeItems.length > 0 ? typeItems.map((item) => [item.id, `${item.label} (${item.slug})`]) : [["", "请先创建认证类型"]]} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="verification-expiry">到期时间（可选）</Label>
              <Input id="verification-expiry" type="datetime-local" value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="verification-evidence">证据引用（可选）</Label>
              <Input id="verification-evidence" value={evidenceReference} onChange={(event) => setEvidenceReference(event.target.value)} maxLength={128} placeholder="case:2026-001" />
            </div>
            <div className="sm:col-span-2">
              <ToggleField id="verification-display-profile" label="显示在公开主页" description={selectedType?.allowsPublicDisplay ? "该类型允许公开；授予后仅公开标签、说明和有效期。" : "该类型不允许公开展示。"} checked={displayOnProfile} onChange={setDisplayOnProfile} disabled={!selectedType?.allowsPublicDisplay} />
            </div>
            <div className="space-y-2 sm:col-span-2">
              <Label htmlFor="verification-grant-reason">授予原因</Label>
              <Textarea id="verification-grant-reason" value={grantReason} onChange={(event) => setGrantReason(event.target.value)} maxLength={500} placeholder="说明已完成的核验和签发依据" />
              <p className="text-xs text-muted-foreground">只写结论与依据类别，不要复制个人信息或证据正文。</p>
            </div>
            <Button type="button" className="sm:col-span-2" onClick={() => grant.mutate()} disabled={!canGrant || grant.isPending}>
              <BadgeCheck className="size-4" />{grant.isPending ? "正在授予…" : "授予认证"}
            </Button>
          </CardContent>
        </Card>
      </div>

      <Card className="rounded-xl">
        <CardHeader>
          <CardTitle className="flex items-center gap-2"><History className="size-4 text-primary" />认证类型与账号历史</CardTitle>
          <CardDescription>公开资料只读取有效且明确允许展示的授予；本区保留到期和撤销历史，但不回显证据引用。</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-5 xl:grid-cols-2">
          <section aria-labelledby="verification-types-heading">
            <h3 id="verification-types-heading" className="mb-3 text-sm font-medium">认证类型</h3>
            {types.isLoading ? <LoadingState label="加载认证类型" /> : types.isError ? (
              <ErrorState title="认证类型加载失败" error={types.error} onRetry={() => void types.refetch()} />
            ) : typeItems.length === 0 ? (
              <EmptyState title="暂无认证类型" description="创建后才能向账号授予认证。" />
            ) : (
              <div className="space-y-2">
                {typeItems.map((item: VerificationType) => (
                  <div key={item.id} className="rounded-lg border p-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <VerificationBadge verification={item} />
                      <Badge variant="outline">{item.allowsPublicDisplay ? "可公开" : "仅内部"}</Badge>
                    </div>
                    <p className="mt-2 text-xs text-muted-foreground">{item.slug} · {item.category === "identity" ? "身份认证" : "特殊认证"}</p>
                  </div>
                ))}
                <PaginationControls
                  hasPrevious={typeCursorStack.length > 1}
                  hasMore={Boolean(types.data?.hasMore && types.data.nextCursor)}
                  onPrevious={() => setTypeCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
                  onNext={() => types.data?.nextCursor && setTypeCursorStack((items) => [...items, types.data?.nextCursor ?? null])}
                />
              </div>
            )}
          </section>

          <section aria-labelledby="verification-history-heading">
            <h3 id="verification-history-heading" className="mb-3 text-sm font-medium">账号授予历史</h3>
            {!normalizedAccountId ? (
              <EmptyState title="输入用户 ID" description="系统不会按模糊名称猜测认证对象。" />
            ) : grants.isLoading ? <LoadingState label="加载账号认证历史" /> : grants.isError ? (
              <ErrorState title="认证历史加载失败" error={grants.error} onRetry={() => void grants.refetch()} />
            ) : (grants.data?.items ?? []).length === 0 ? (
              <EmptyState title="该账号暂无认证" />
            ) : (
              <div className="space-y-2">
                {grants.data?.items?.map((item) => (
                  <div key={item.id} className="rounded-lg border p-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{item.label}</span>
                      <AdminStatusBadge value={item.status} />
                      {item.displayOnProfile ? <Badge variant="outline">公开展示</Badge> : null}
                      {item.hasEvidence ? <Badge variant="outline">已关联证据</Badge> : null}
                    </div>
                    <p className="mt-2 text-xs leading-5 text-muted-foreground">
                      签发 {formatUnixTime(item.issuedAt)}
                      {item.expiresAt ? ` · 到期 ${formatUnixTime(item.expiresAt)}` : " · 长期有效"}
                    </p>
                    <p className="mt-1 text-xs leading-5 text-muted-foreground">授予原因：{item.issueReason}</p>
                    {item.revokeReason ? <p className="mt-1 text-xs leading-5 text-muted-foreground">撤销原因：{item.revokeReason}</p> : null}
                    {item.status === "active" ? (
                      <Button type="button" variant="destructive" size="sm" className="mt-3" onClick={() => setRevoking(item)}>
                        <ShieldX className="size-4" />撤销认证
                      </Button>
                    ) : null}
                  </div>
                ))}
                <PaginationControls
                  hasPrevious={grantCursorStack.length > 1}
                  hasMore={Boolean(grants.data?.hasMore && grants.data.nextCursor)}
                  onPrevious={() => setGrantCursorStack((items) => items.length > 1 ? items.slice(0, -1) : items)}
                  onNext={() => grants.data?.nextCursor && setGrantCursorStack((items) => [...items, grants.data?.nextCursor ?? null])}
                />
              </div>
            )}
          </section>
        </CardContent>
      </Card>

      <ReasonDialog
        open={Boolean(revoking)}
        onOpenChange={(open) => !open && setRevoking(null)}
        title={`撤销“${revoking?.label ?? ""}”认证`}
        description="撤销会立即停止公开展示并保留签发、到期和撤销历史。该操作不会删除成就徽章或改变账号角色。"
        confirmLabel="确认撤销认证"
        destructive
        isPending={revoke.isPending}
        onConfirm={(reason) => revoking && revoke.mutate({ item: revoking, reason })}
      />
    </div>
  );
}

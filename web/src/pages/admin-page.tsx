import {
  Activity,
  Award,
  BadgeCheck,
  FileClock,
  Gavel,
  LayoutDashboard,
  Megaphone,
  RectangleHorizontal,
  Scale,
  Settings2,
  ShieldAlert,
  Tags,
  Users,
} from "lucide-react";
import * as React from "react";
import { useSearchParams } from "react-router";

import { ActivityPolicyPanel } from "@/components/admin/activity-policy-panel";
import { AchievementsPanel } from "@/components/admin/achievements-panel";
import { AdminShell, type AdminNavigationItem } from "@/components/admin/admin-shell";
import { AnnouncementsPanel } from "@/components/admin/announcements-panel";
import { AppealsPanel } from "@/components/admin/appeals-panel";
import { AuditPanel } from "@/components/admin/audit-panel";
import { CreditIntegrityPanel } from "@/components/admin/credit-integrity-panel";
import {
  ADMIN_CAPABILITIES,
  capabilitiesForAccount,
  hasCapability,
} from "@/components/admin/capabilities";
import { ModerationPanel } from "@/components/admin/moderation-panel";
import { OverviewPanel } from "@/components/admin/overview-panel";
import { PromotionsPanel } from "@/components/admin/promotions-panel";
import { ResourcesPanel } from "@/components/admin/resources-panel";
import { SystemPanel } from "@/components/admin/system-panel";
import { UsersPanel } from "@/components/admin/users-panel";
import { VerificationsPanel } from "@/components/admin/verifications-panel";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState, LoadingState } from "@/components/common/states";
import { Badge } from "@/components/ui/badge";
import { useAuth } from "@/context/auth-provider";

type AdminSection = "overview" | "users" | "moderation" | "appeals" | "resources" | "activity" | "announcements" | "promotions" | "achievements" | "verifications" | "credit-integrity" | "audit" | "system";

function isAdminSection(value: string | null): value is AdminSection {
  return ["overview", "users", "moderation", "appeals", "resources", "activity", "announcements", "promotions", "achievements", "verifications", "credit-integrity", "audit", "system"].includes(value ?? "");
}

export function AdminPage() {
  const { account, isAuthenticated, isLoading } = useAuth();
  const [searchParams, setSearchParams] = useSearchParams();
  const capabilities = React.useMemo(() => capabilitiesForAccount(account), [account]);
  const [active, setActive] = React.useState<AdminSection>(() => {
    const requested = searchParams.get("section");
    return isAdminSection(requested) ? requested : "overview";
  });
  const canSearchUsers = hasCapability(capabilities, ADMIN_CAPABILITIES.searchUsers);
  const canManageUsers = canSearchUsers
    || hasCapability(capabilities, ADMIN_CAPABILITIES.inviteUsers)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.changeRoles)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.silenceUsers)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.suspendUsers);
  const canModerate = hasCapability(capabilities, ADMIN_CAPABILITIES.moderateContent);
  const canReviewAppeals = hasCapability(capabilities, ADMIN_CAPABILITIES.reviewAppeals);
  const canManageActivity = hasCapability(capabilities, ADMIN_CAPABILITIES.manageActivity);
  const canManageAnnouncements = hasCapability(capabilities, ADMIN_CAPABILITIES.manageAnnouncements);
  const canManagePromotions = hasCapability(capabilities, ADMIN_CAPABILITIES.managePromotions);
  const canManageBadges = hasCapability(capabilities, ADMIN_CAPABILITIES.manageBadges);
  const canManageVerifications = hasCapability(capabilities, ADMIN_CAPABILITIES.manageVerifications);
  const canReadAudit = hasCapability(capabilities, ADMIN_CAPABILITIES.readAudit);
  const canManageSettings = hasCapability(capabilities, ADMIN_CAPABILITIES.managePlatform);
  const canRunJobs = hasCapability(capabilities, ADMIN_CAPABILITIES.runOperations);
  const canManageCreditIntegrity = hasCapability(capabilities, ADMIN_CAPABILITIES.manageCreditIntegrity);
  const canManageResources = canModerate
    || hasCapability(capabilities, ADMIN_CAPABILITIES.manageCourses)
    || hasCapability(capabilities, ADMIN_CAPABILITIES.manageCommunity);

  const items = React.useMemo<Array<AdminNavigationItem & { id: AdminSection }>>(() => {
    const next: Array<AdminNavigationItem & { id: AdminSection }> = [];
    if (canSearchUsers) next.push({ id: "overview", label: "概览", description: "队列与社区状态", icon: LayoutDashboard });
    if (canManageUsers) next.push({ id: "users", label: "用户", description: "邀请、角色与限制", icon: Users });
    if (canModerate) next.push({ id: "moderation", label: "审核", description: "论坛、点评与私信举报", icon: ShieldAlert });
    if (canReviewAppeals) next.push({ id: "appeals", label: "申诉", description: "独立复核与恢复", icon: Gavel });
    if (canManageResources) next.push({ id: "resources", label: "内容资源", description: "媒体、课程与社区结构", icon: Tags });
    if (canManageActivity) next.push({ id: "activity", label: "活跃度", description: "权重策略与版本", icon: Activity });
    if (canManageAnnouncements) next.push({ id: "announcements", label: "公告", description: "发布与修订", icon: Megaphone });
    if (canManagePromotions) next.push({ id: "promotions", label: "推广", description: "素材、排期与排序", icon: RectangleHorizontal });
    if (canManageBadges) next.push({ id: "achievements", label: "成就", description: "贡献定义与授予", icon: Award });
    if (canManageVerifications) next.push({ id: "verifications", label: "认证", description: "身份与特殊标识", icon: BadgeCheck });
    if (canManageCreditIntegrity) next.push({ id: "credit-integrity", label: "积分完整性", description: "只读账本与钱包对账", icon: Scale });
    if (canReadAudit) next.push({ id: "audit", label: "审计", description: "操作记录", icon: FileClock });
    if (canManageSettings || canRunJobs) next.push({ id: "system", label: "平台", description: "系统设置与维护", icon: Settings2 });
    return next;
  }, [canManageActivity, canManageAnnouncements, canManageBadges, canManageCreditIntegrity, canManagePromotions, canManageResources, canManageSettings, canManageUsers, canManageVerifications, canModerate, canReadAudit, canReviewAppeals, canRunJobs, canSearchUsers]);

  React.useEffect(() => {
    const requested = searchParams.get("section");
    if (
      isAdminSection(requested)
      && items.some((item) => item.id === requested)
      && requested !== active
    ) {
      setActive(requested);
    }
  }, [active, items, searchParams]);

  React.useEffect(() => {
    if (!items.some((item) => item.id === active) && items[0]) {
      setActive(items[0].id);
    }
  }, [active, items]);

  const visibleActive = items.some((item) => item.id === active)
    ? active
    : items[0]?.id ?? "overview";

  if (isLoading) {
    return <LoadingState label="确认管理权限" />;
  }
  if (!isAuthenticated || items.length === 0) {
    return <EmptyState title="没有管理权限" description="当前账号没有可访问的管理功能。" />;
  }

  let panel: React.ReactNode;
  switch (visibleActive) {
    case "users":
      panel = <UsersPanel capabilities={capabilities} initialQuery={searchParams.get("q") ?? ""} />;
      break;
    case "moderation":
      panel = <ModerationPanel />;
      break;
    case "appeals":
      panel = <AppealsPanel />;
      break;
    case "activity":
      panel = <ActivityPolicyPanel />;
      break;
    case "resources":
      panel = <ResourcesPanel capabilities={capabilities} />;
      break;
    case "announcements":
      panel = <AnnouncementsPanel />;
      break;
    case "promotions":
      panel = <PromotionsPanel />;
      break;
    case "achievements":
      panel = <AchievementsPanel initialAccountId={searchParams.get("account") ?? ""} />;
      break;
    case "verifications":
      panel = <VerificationsPanel initialAccountId={searchParams.get("account") ?? ""} />;
      break;
    case "credit-integrity":
      panel = <CreditIntegrityPanel />;
      break;
    case "audit":
      panel = <AuditPanel />;
      break;
    case "system":
      panel = <SystemPanel canManageSettings={canManageSettings} canRunJobs={canRunJobs} />;
      break;
    default:
      panel = <OverviewPanel />;
  }

  return (
    <div>
      <PageHeader
        title="管理后台"
        description="审核内容、管理账号与处理平台事务。不同权限会显示不同功能入口。"
        actions={
          <>
            <Badge variant="secondary">{account?.role === "admin" ? "管理员" : "版主"}</Badge>
            <Badge variant="outline">{capabilities.size} 项能力</Badge>
          </>
        }
      />
      <AdminShell
        items={items}
        active={visibleActive}
        onActiveChange={(id) => {
          const next = id as AdminSection;
          setActive(next);
          const params = new URLSearchParams(searchParams);
          params.set("section", next);
          setSearchParams(params, { replace: true });
        }}
      >
        {panel}
      </AdminShell>
    </div>
  );
}

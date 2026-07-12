import { Link } from "react-router";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/common/states";
import { AccountDataSettings } from "@/components/settings/account-data-settings";
import { NotificationSettings } from "@/components/settings/notification-settings";
import { ProfileMediaSettings } from "@/components/settings/profile-media-settings";
import { ProfilePrivacySettings } from "@/components/settings/profile-privacy-settings";
import { SecuritySettings } from "@/components/settings/security-settings";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/context/auth-provider";

export function SettingsPage() {
  const { account, isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <EmptyState title="登录后修改设置" />;
  }

  if (account?.onboardingRequired) {
    return (
      <div className="mx-auto max-w-2xl">
        <PageHeader eyebrow="Account security" title="账号与安全" description="即使尚未完成入门设置，你仍然可以管理设备、修改密码、导出或关闭账号。" />
        <div className="space-y-4">
          <Button asChild variant="outline"><Link to="/onboarding">返回完成入门设置</Link></Button>
          <SecuritySettings />
          <AccountDataSettings />
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-2xl">
      <PageHeader eyebrow="Settings" title="设置" description="管理公开资料、隐私、通知与账号安全。" />
      <div className="space-y-4">
        <ProfileMediaSettings />
        <ProfilePrivacySettings />
        <NotificationSettings />
        <SecuritySettings />
        <AccountDataSettings />
      </div>
    </div>
  );
}

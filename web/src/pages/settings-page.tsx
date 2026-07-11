import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/common/states";
import { NotificationSettings } from "@/components/settings/notification-settings";
import { ProfileMediaSettings } from "@/components/settings/profile-media-settings";
import { ProfilePrivacySettings } from "@/components/settings/profile-privacy-settings";
import { SecuritySettings } from "@/components/settings/security-settings";
import { useAuth } from "@/context/auth-provider";

export function SettingsPage() {
  const { isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <EmptyState title="登录后修改设置" />;
  }

  return (
    <div className="max-w-2xl">
      <PageHeader eyebrow="Settings" title="设置" description="管理公开资料、隐私、通知与账号安全。" />
      <div className="space-y-4">
        <ProfileMediaSettings />
        <ProfilePrivacySettings />
        <NotificationSettings />
        <SecuritySettings />
      </div>
    </div>
  );
}

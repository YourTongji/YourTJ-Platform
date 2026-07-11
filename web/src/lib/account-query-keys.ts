const SIGNED_OUT_SCOPE = "signed-out";

export function accountQueryScope(accountId: string | undefined | null) {
  return accountId ?? SIGNED_OUT_SCOPE;
}

export const accountQueryKeys = {
  notifications: (accountId: string | undefined | null) =>
    ["notifications", accountQueryScope(accountId)] as const,
  notificationCount: (accountId: string | undefined | null) =>
    ["notification-count", accountQueryScope(accountId)] as const,
  governanceNotices: (accountId: string | undefined | null) =>
    ["governance-notices", accountQueryScope(accountId)] as const,
  governanceNoticeCount: (accountId: string | undefined | null) =>
    ["governance-notice-count", accountQueryScope(accountId)] as const,
  notificationPreferences: (accountId: string | undefined | null) =>
    ["notification-prefs", accountQueryScope(accountId)] as const,
  directMessages: (accountId: string | undefined | null) =>
    ["dm", accountQueryScope(accountId)] as const,
  directMessageCount: (accountId: string | undefined | null) =>
    ["dm-unread-count", accountQueryScope(accountId)] as const,
  ignoredUsers: (accountId: string | undefined | null) =>
    ["ignores", accountQueryScope(accountId)] as const,
};

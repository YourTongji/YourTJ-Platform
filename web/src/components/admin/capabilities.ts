import type { Account } from "@/lib/api/types";

export const ADMIN_CAPABILITIES = {
  moderateContent: "moderation.content",
  searchUsers: "users.search",
  silenceUsers: "users.silence",
  readAudit: "audit.read",
  inviteUsers: "users.invite",
  changeRoles: "users.roles",
  suspendUsers: "users.suspend",
  manageCommunity: "community.manage",
  manageCourses: "courses.manage",
  managePlatform: "platform.settings",
  manageActivity: "activity.policy",
  manageAnnouncements: "announcements.manage",
  managePromotions: "promotions.manage",
  runOperations: "operations.jobs",
} as const;

export type AdminCapability = (typeof ADMIN_CAPABILITIES)[keyof typeof ADMIN_CAPABILITIES];

const moderatorFallback: AdminCapability[] = [
  ADMIN_CAPABILITIES.moderateContent,
  ADMIN_CAPABILITIES.searchUsers,
  ADMIN_CAPABILITIES.silenceUsers,
  ADMIN_CAPABILITIES.readAudit,
];

const administratorFallback: AdminCapability[] = Object.values(ADMIN_CAPABILITIES);

/** Uses server-issued capabilities and falls back only for accounts saved before capability rollout. */
export function capabilitiesForAccount(account: Account | null) {
  if (account?.capabilities?.length) {
    return new Set(account.capabilities);
  }
  if (account?.role === "admin") {
    return new Set(administratorFallback);
  }
  if (account?.role === "mod") {
    return new Set(moderatorFallback);
  }
  return new Set<string>();
}

export function hasCapability(capabilities: Set<string>, capability: AdminCapability) {
  return capabilities.has(capability);
}

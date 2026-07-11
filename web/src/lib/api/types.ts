import type { components } from "./schema";

export type Schema = components["schemas"];

export type Account = Schema["Account"] & { trustLevel?: number };
export type AccountLifecycleState = Schema["AccountLifecycleState"];
export type AccountLifecycle = Schema["AccountLifecycle"];
export type AccountLifecycleMutation = Schema["AccountLifecycleMutation"];
export type AccountLifecycleMutationInput = Schema["AccountLifecycleMutationInput"];
export type RecoveryCredential = Schema["RecoveryCredential"];
export type OnboardingState = Schema["OnboardingState"];
export type OnboardingCompleteInput = Schema["OnboardingCompleteInput"];
export type DataExportJob = Schema["DataExportJob"];
export type DataExportStatus = Schema["DataExportStatus"];
export type DataExportDownloadGrant = Schema["DataExportDownloadGrant"];
export type AccountDataExport = Schema["AccountDataExport"];
export type ProfileVisibility = Schema["ProfileVisibility"];
export type ActivityVisibility = Schema["ActivityVisibility"];
export type AuthTokens = Schema["AuthTokens"];
export type AppealAccessToken = Schema["AppealAccessToken"];
export type Appeal = Schema["Appeal"];
export type AdminAppeal = Schema["AdminAppeal"];
export type AppealStatus = Schema["AppealStatus"];
export type GovernanceNotice = Schema["GovernanceNotice"];
export type DeviceSession = Schema["Session"];
export type DeviceSessionPage = Schema["SessionPage"];
export type EmailCodePurpose = Schema["EmailCodePurpose"];
export type RecentAuthMethod = Schema["RecentAuthMethod"];
export type RecentAuthStatus = Schema["RecentAuthStatus"];
export type RecentAuthVerifyInput = Schema["RecentAuthVerifyInput"];
export type ActivityCalendar = Schema["ActivityCalendar"];
export type ActivityDay = Schema["ActivityDay"];
export type ActivityPolicy = Schema["ActivityPolicy"];
export type ActivityPolicyUpdateInput = Schema["ActivityPolicyUpdateInput"];
export type Achievement = Schema["Achievement"];
export type AchievementCreateInput = Schema["AchievementCreateInput"];
export type AchievementUpdateInput = Schema["AchievementUpdateInput"];
export type AchievementGrant = Schema["AchievementGrant"];
export type AchievementGrantInput = Schema["AchievementGrantInput"];
export type AchievementEvent = Schema["AchievementEvent"];
export type AchievementIcon = Schema["AchievementIcon"];
export type AchievementStatus = Schema["AchievementStatus"];
export type AdminAuditEvent = Schema["AdminAuditEvent"];
export interface AdminBoardCreateInput {
  slug: string;
  name: string;
  description?: string;
  position?: number;
  isLocked?: boolean;
  minTrustToPost?: number;
  isQa?: boolean;
  reason: string;
}
export type AdminBoardUpdateInput = Partial<Omit<AdminBoardCreateInput, "reason">> & {
  reason: string;
};
export interface AdminCourseCreateInput {
  code: string;
  name: string;
  credit?: number;
  department?: string;
  teacherName?: string;
  reason: string;
}
export type AdminCourseUpdateInput = Partial<Omit<AdminCourseCreateInput, "reason">> & {
  reason: string;
};
export interface SettingUpdateInput {
  value: string;
  reason: string;
}
export interface AdminTagCreateInput {
  slug: string;
  name: string;
  description?: string;
  reason: string;
}
export type AdminTagUpdateInput = Partial<Omit<AdminTagCreateInput, "reason">> & {
  reason: string;
};
export type AdminOverview = Schema["AdminOverview"];
export type AdminUser = Schema["AdminUser"];
export type AdminUserInviteInput = Schema["AdminUserInviteInput"];
export type Announcement = Schema["Announcement"];
export type AdminVersionedArchiveInput = Schema["AdminVersionedArchiveInput"];
export type AnnouncementCreateInput = Schema["AnnouncementCreateInput"];
export type AnnouncementReceipt = Schema["AnnouncementReceipt"];
export type AnnouncementReceiptInput = Schema["AnnouncementReceiptInput"];
export type AnnouncementRevision = Schema["AnnouncementRevision"];
export type AnnouncementUpdateInput = Schema["AnnouncementUpdateInput"];
export type Board = Schema["Board"];
export type Comment = Schema["Comment"];
export type ContentFormat = Schema["ContentFormat"];
export type ForumAttachment = Schema["ForumAttachment"];
export type Course = Schema["Course"];
export type CourseDetail = Schema["CourseDetail"];
export type Department = Schema["Department"];
export type DmReport = Schema["DmReport"];
export type Draft = Schema["DraftOutput"];
export type DraftPayload = Schema["ForumDraftPayload"];
export type DraftSaveInput = Schema["DraftSaveInput"];
export type LedgerEntry = Schema["LedgerEntry"];
export type LedgerVerify = Schema["LedgerVerify"];
export type CreditReconciliationRun = Schema["CreditReconciliationRun"];
export type CreditReconciliationStats = Schema["CreditReconciliationStats"];
export type CreditReconciliationWallet = Schema["CreditReconciliationWallet"];
export type Notification = Schema["Notification"];
export type NotificationPreferences = Schema["NotificationPreferences"];
export type NotificationOutboxEvent = Schema["NotificationOutboxEvent"];
export type NotificationOutboxState = NotificationOutboxEvent["state"];
export type Product = Schema["Product"];
export type Promotion = Schema["Promotion"];
export type PromotionCreateInput = Schema["PromotionCreateInput"];
export type PromotionMetrics = Schema["PromotionMetrics"];
export type PromotionUpdateInput = Schema["PromotionUpdateInput"];
export type Purchase = Schema["Purchase"];
export type Review = Schema["Review"];
export type ReviewReport = Schema["Report"];
export type Sanction = Schema["Sanction"];
export type SearchResult = Schema["SearchResult"];
export type UserSearchHit = Schema["UserSearchHit"];
export type BoardSearchHit = Schema["BoardSearchHit"];
export type TagSearchHit = Schema["TagSearchHit"];
export type SigningIntent = Schema["SigningIntent"];
export type Setting = Schema["Setting"];
export type Tag = Schema["Tag"];
export type Task = Schema["Task"];
export type Thread = Schema["Thread"];
export type ThreadDetail = Schema["ThreadDetail"];
export type ThreadFeed = Schema["ThreadFeed"];
export type UserProfile = Schema["UserProfile"] & { id: string };
export type UserThread = Schema["UserThread"];
export type UserComment = Schema["UserComment"];
export type UserSummary = Schema["UserSummary"];
export type UserRelationship = Schema["UserRelationship"];
export type MyProfile = Schema["MyProfile"];
export type MediaUsage = Schema["MediaUsage"];
export type MediaRetentionHoldInput = Schema["MediaRetentionHoldInput"];
export type MediaRetentionHold = Schema["MediaRetentionHold"];
export type MediaDeletionJob = Schema["MediaDeletionJob"];
export type ModerationPreviewGrant = Schema["ModerationPreviewGrant"];
export type MyUpload = Schema["MyUpload"];
export type ProfileUpdateInput = Schema["ProfileUpdateInput"];
export type ProfilePrivacy = Schema["ProfilePrivacy"];
export type ProfilePrivacyUpdateInput = Schema["ProfilePrivacyUpdateInput"];
export type Upload = Schema["Upload"];
export type UploadCredentials = Schema["UploadCredentials"];
export type UploadUrl = Schema["UploadUrl"];
export type PublicVerification = Schema["PublicVerification"];
export type VerificationCategory = Schema["VerificationCategory"];
export type VerificationIcon = Schema["VerificationIcon"];
export type VerificationBadgeVariant = Schema["VerificationBadgeVariant"];
export type VerificationType = Schema["VerificationType"];
export type VerificationTypeInput = Schema["VerificationTypeInput"];
export type VerificationGrant = Schema["VerificationGrant"];
export type VerificationGrantInput = Schema["VerificationGrantInput"];
export type WatchedWord = Schema["WatchedWord"];
export interface WatchedWordInput {
  word: string;
  action: "block" | "censor" | "queue";
  reason: string;
}
export type Wallet = Schema["Wallet"];

export type Calendar = Schema["Calendar"];
export type Campus = Schema["Campus"];
export type CourseNature = Schema["CourseNature"];
export type Faculty = Schema["Faculty"];
export type Major = Schema["Major"];
export type SelectionCourse = Schema["SelectionCourse"] & {
  credit?: number | null;
  campusId?: string | null;
  teacherNames?: string[];
};
export type TimeSlot = Schema["TimeSlot"] & {
  weeks?: string | null;
  location?: string | null;
};
export type LatestUpdate = Schema["LatestUpdate"] & {
  updatedAt?: string | null;
};

export type PollOption = Schema["PollOption"];
export type Poll = Schema["Poll"];
export type ThreadDetailWithPoll = ThreadDetail;

export type DmConversation = Schema["DmConversation"];
export type DmCounts = Schema["DmCounts"];
export type DmMessage = Schema["DmMessage"];
export type IgnoreUser = Schema["IgnoreUser"];
export type DmReportReason = Schema["DmReportInput"]["reason"];

export interface Bookmark {
  targetType?: string;
  targetId?: string;
  note?: string | null;
  createdAt?: number;
}

export interface AdminForumFlag {
  id: string;
  targetType: "thread" | "comment";
  targetId: string;
  reporterId: string;
  reason: string;
  note?: string | null;
  weight: number;
  status: string;
  authorHandle?: string | null;
  targetTitle?: string | null;
  contentExcerpt?: string | null;
  createdAt: number;
}

export interface Page<T> {
  items?: T[];
  nextCursor?: string | null;
  hasMore?: boolean;
}

export interface ApiErrorBody {
  error?: {
    code?: string;
    message?: string;
    details?: Record<string, unknown>;
  };
}

import type { components } from "./schema";

export type Schema = components["schemas"];

export type Account = Schema["Account"] & { trustLevel?: number };
export type AuthTokens = Schema["AuthTokens"];
export type DeviceSession = Schema["Session"];
export type DeviceSessionPage = Schema["SessionPage"];
export type EmailCodePurpose = Schema["EmailCodePurpose"];
export type ActivityCalendar = Schema["ActivityCalendar"];
export type ActivityDay = Schema["ActivityDay"];
export type ActivityPolicy = Schema["ActivityPolicy"];
export type ActivityPolicyUpdateInput = Schema["ActivityPolicyUpdateInput"];
export type AdminAuditEvent = Schema["AdminAuditEvent"];
export interface AdminBoardCreateInput {
  slug: string;
  name: string;
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
export type AnnouncementInput = Schema["AnnouncementInput"] & { reason: string };
export type Board = Schema["Board"];
export type Comment = Schema["Comment"];
export type Course = Schema["Course"];
export type CourseDetail = Schema["CourseDetail"];
export type Department = Schema["Department"];
export type DmReport = Schema["DmReport"];
export type LedgerEntry = Schema["LedgerEntry"];
export type LedgerVerify = Schema["LedgerVerify"];
export type Notification = Schema["Notification"];
export type NotificationPreferences = Schema["NotificationPreferences"];
export type Product = Schema["Product"];
export type Purchase = Schema["Purchase"];
export type Review = Schema["Review"];
export type ReviewReport = Schema["Report"];
export type Sanction = Schema["Sanction"];
export type SearchResult = Schema["SearchResult"];
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
export type Upload = Schema["Upload"];
export type UploadCredentials = Schema["UploadCredentials"];
export type UploadUrl = Schema["UploadUrl"];
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

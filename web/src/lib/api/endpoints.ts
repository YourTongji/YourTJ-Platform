import { apiRequest } from "./client";
import type {
  Account,
  Achievement,
  AchievementCreateInput,
  AchievementEvent,
  AchievementGrant,
  AchievementGrantInput,
  AchievementUpdateInput,
  ActivityPolicy,
  ActivityPolicyUpdateInput,
  ActivityCalendar,
  AdminAuditEvent,
  AdminBoardCreateInput,
  AdminBoardUpdateInput,
  AdminCourseCreateInput,
  AdminCourseUpdateInput,
  AdminForumFlag,
  AdminOverview,
  AdminUser,
  AdminUserInviteInput,
  AdminTagCreateInput,
  AdminTagUpdateInput,
  Announcement,
  AdminVersionedArchiveInput,
  AnnouncementCreateInput,
  AnnouncementReceipt,
  AnnouncementReceiptInput,
  AnnouncementRevision,
  AnnouncementUpdateInput,
  AuthTokens,
  Board,
  Bookmark,
  Calendar,
  Campus,
  Comment,
  ContentFormat,
  Course,
  CourseDetail,
  CourseNature,
  CreditReconciliationRun,
  CreditReconciliationStats,
  CreditReconciliationWallet,
  Department,
  DeviceSessionPage,
  DmConversation,
  DmCounts,
  DmMessage,
  DmReportReason,
  DmReport,
  Draft,
  DraftSaveInput,
  EmailCodePurpose,
  Faculty,
  LatestUpdate,
  LedgerEntry,
  LedgerVerify,
  Major,
  MediaUsage,
  Notification,
  NotificationPreferences,
  Page,
  Poll,
  Product,
  Promotion,
  PromotionCreateInput,
  PromotionMetrics,
  PromotionUpdateInput,
  Purchase,
  Review,
  ReviewReport,
  Sanction,
  SearchResult,
  SigningIntent,
  SelectionCourse,
  Setting,
  SettingUpdateInput,
  Tag,
  Task,
  ThreadDetailWithPoll,
  ThreadFeed,
  TimeSlot,
  Upload,
  UploadCredentials,
  UploadUrl,
  VerificationGrant,
  VerificationGrantInput,
  VerificationType,
  VerificationTypeInput,
  IgnoreUser,
  MyProfile,
  MyUpload,
  ProfilePrivacy,
  ProfileUpdateInput,
  RecentAuthStatus,
  RecentAuthVerifyInput,
  UserComment,
  UserProfile,
  UserRelationship,
  UserSummary,
  UserThread,
  WatchedWord,
  WatchedWordInput,
  Wallet,
} from "./types";

export interface WalletAuthorization {
  idempotencyKey: string;
  intentId: string;
  signature: string;
}

function walletHeaders(authorization: WalletAuthorization) {
  return {
    "Idempotency-Key": authorization.idempotencyKey,
    "X-Wallet-Intent": authorization.intentId,
    "X-Wallet-Sig": authorization.signature,
  };
}

export const api = {
  requestEmailCode(email: string, captchaToken: string, purpose?: EmailCodePurpose) {
    return apiRequest<void>("/auth/email/request-code", {
      method: "POST",
      body: { email, captchaToken, purpose },
      auth: false,
    });
  },

  verifyEmail(input: {
    email: string;
    code: string;
    purpose?: EmailCodePurpose;
    handle?: string;
    password?: string;
  }) {
    return apiRequest<AuthTokens>("/auth/email/verify", {
      method: "POST",
      body: input,
      auth: false,
    });
  },

  passwordLogin(input: { email: string; password: string }) {
    return apiRequest<AuthTokens>("/auth/password/login", {
      method: "POST",
      body: input,
      auth: false,
    });
  },

  passwordForgot(email: string, captchaToken: string) {
    return apiRequest<void>("/auth/password/forgot", {
      method: "POST",
      body: { email, captchaToken },
      auth: false,
    });
  },

  passwordReset(input: { email: string; code: string; newPassword: string }) {
    return apiRequest<void>("/auth/password/reset", {
      method: "POST",
      body: input,
      auth: false,
    });
  },

  passwordChange(input: { currentPassword: string; newPassword: string }) {
    return apiRequest<void>("/auth/password/change", { method: "POST", body: input });
  },

  recentAuthStatus() {
    return apiRequest<RecentAuthStatus>("/auth/recent-auth");
  },

  requestRecentAuthCode() {
    return apiRequest<void>("/auth/recent-auth/email/request-code", { method: "POST" });
  },

  verifyRecentAuth(input: RecentAuthVerifyInput) {
    return apiRequest<RecentAuthStatus>("/auth/recent-auth/verify", {
      method: "POST",
      body: input,
    });
  },

  logout() {
    return apiRequest<void>("/auth/logout", { method: "POST" });
  },

  logoutAll() {
    return apiRequest<void>("/auth/logout-all", { method: "POST" });
  },

  sessions(cursor?: string | null) {
    return apiRequest<DeviceSessionPage>("/me/sessions", { query: { cursor, limit: 30 } });
  },

  revokeSession(id: string) {
    return apiRequest<void>(`/me/sessions/${encodeURIComponent(id)}`, { method: "DELETE" });
  },

  revokeOtherSessions() {
    return apiRequest<void>("/me/sessions/revoke-others", { method: "POST" });
  },

  me() {
    return apiRequest<Account>("/me");
  },

  updateMe(input: { handle?: string }) {
    return apiRequest<Account>("/me", { method: "PATCH", body: input });
  },

  myProfile() {
    return apiRequest<MyProfile>("/me/profile");
  },

  updateMyProfile(input: ProfileUpdateInput) {
    return apiRequest<MyProfile>("/me/profile", { method: "PUT", body: input });
  },

  myPrivacy() {
    return apiRequest<ProfilePrivacy>("/me/privacy");
  },

  updateMyPrivacy(input: ProfilePrivacy) {
    return apiRequest<ProfilePrivacy>("/me/privacy", { method: "PUT", body: input });
  },

  myActivity(from?: string, to?: string) {
    return apiRequest<ActivityCalendar>("/me/activity", { query: { from, to } });
  },

  drafts() {
    return apiRequest<Page<Draft>>("/me/drafts");
  },

  draft(draftKey: string) {
    return apiRequest<Draft>(`/me/drafts/${encodeURIComponent(draftKey)}`);
  },

  saveDraft(input: DraftSaveInput) {
    return apiRequest<Draft>("/me/drafts", { method: "PUT", body: input });
  },

  deleteDraft(draftKey: string) {
    return apiRequest<void>(`/me/drafts/${encodeURIComponent(draftKey)}`, {
      method: "DELETE",
    });
  },

  publicUser(handle: string) {
    return apiRequest<UserProfile>(`/users/${encodeURIComponent(handle)}`, { auth: "optional" });
  },

  userThreads(handle: string, cursor?: string | null) {
    return apiRequest<Page<UserThread>>(`/users/${encodeURIComponent(handle)}/threads`, {
      query: { cursor, limit: 20 },
      auth: "optional",
    });
  },

  userComments(handle: string, cursor?: string | null) {
    return apiRequest<Page<UserComment>>(`/users/${encodeURIComponent(handle)}/comments`, {
      query: { cursor, limit: 20 },
      auth: "optional",
    });
  },

  userRelationship(handle: string) {
    return apiRequest<UserRelationship>(`/users/${encodeURIComponent(handle)}/relationship`);
  },

  followUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/follow`, { method: "PUT" });
  },

  unfollowUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/follow`, { method: "DELETE" });
  },

  removeFollower(handle: string) {
    return apiRequest<void>(`/me/followers/${encodeURIComponent(handle)}`, {
      method: "DELETE",
    });
  },

  userFollowers(handle: string, cursor?: string | null) {
    return apiRequest<Page<UserSummary>>(`/users/${encodeURIComponent(handle)}/followers`, {
      query: { cursor, limit: 30 },
      auth: "optional",
    });
  },

  userFollowing(handle: string, cursor?: string | null) {
    return apiRequest<Page<UserSummary>>(`/users/${encodeURIComponent(handle)}/following`, {
      query: { cursor, limit: 30 },
      auth: "optional",
    });
  },

  muteUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/mute`, { method: "PUT" });
  },

  unmuteUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/mute`, { method: "DELETE" });
  },

  blockUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/block`, { method: "PUT" });
  },

  unblockUser(handle: string) {
    return apiRequest<void>(`/users/${encodeURIComponent(handle)}/block`, { method: "DELETE" });
  },

  announcements() {
    return apiRequest<Announcement[]>("/announcements");
  },

  unreadAnnouncements() {
    return apiRequest<Announcement[]>("/announcements/unread");
  },

  recordAnnouncementReceipt(id: string, body: AnnouncementReceiptInput) {
    return apiRequest<AnnouncementReceipt>(`/announcements/${encodeURIComponent(id)}/receipt`, {
      method: "POST",
      body,
    });
  },

  promotions(placement?: Promotion["placement"]) {
    return apiRequest<Promotion[]>("/promotions", { query: { placement } });
  },

  recordPromotionEvent(id: string, eventType: "impression" | "click", trackingToken: string) {
    return apiRequest<void>(`/promotions/${encodeURIComponent(id)}/events`, {
      method: "POST",
      auth: false,
      keepalive: true,
      body: { eventType, trackingToken },
    });
  },

  settings() {
    return apiRequest<Setting[]>("/settings", { auth: false });
  },

  mediaUploadCredentials(kind: "image" | "file", contentType: string, usage?: MediaUsage) {
    return apiRequest<UploadCredentials>("/media/upload-credentials", {
      method: "POST",
      body: { kind, contentType, usage },
    });
  },

  mediaUrl(id: string) {
    return apiRequest<UploadUrl>(`/media/${encodeURIComponent(id)}/url`);
  },

  myMediaUploads(usage?: MediaUsage, cursor?: string | null) {
    return apiRequest<Page<MyUpload>>("/me/media/uploads", {
      query: { usage, cursor, limit: 12 },
    });
  },

  myMediaUpload(id: string) {
    return apiRequest<MyUpload>(`/me/media/uploads/${encodeURIComponent(id)}`);
  },

  bindMyProfileMedia(slot: "avatar" | "banner", assetId: string) {
    return apiRequest<void>(`/me/profile/${slot}`, { method: "PUT", body: { assetId } });
  },

  clearMyProfileMedia(slot: "avatar" | "banner") {
    return apiRequest<void>(`/me/profile/${slot}`, { method: "DELETE" });
  },

  departments() {
    return apiRequest<Department[]>("/departments", { auth: false });
  },

  courses(query: { dept?: string; sort?: "hot" | "rating" | "new"; cursor?: string | null }) {
    return apiRequest<Page<Course>>("/courses", {
      query: { ...query, limit: 24 },
      auth: false,
    });
  },

  course(id: string) {
    return apiRequest<CourseDetail>(`/courses/${encodeURIComponent(id)}`, { auth: false });
  },

  relatedCourses(id: string) {
    return apiRequest<Course[]>(`/courses/${encodeURIComponent(id)}/related`, { auth: false });
  },

  courseAiSummary(id: string) {
    return apiRequest<{ summary?: string; updatedAt?: number; model?: string }>(
      `/courses/${encodeURIComponent(id)}/ai-summary`,
      { auth: false },
    );
  },

  courseReviews(id: string, query: { sort?: "hot" | "new"; cursor?: string | null }) {
    return apiRequest<Page<Review>>(`/courses/${encodeURIComponent(id)}/reviews`, {
      query: { ...query, limit: 20 },
      auth: false,
    });
  },

  createReview(
    id: string,
    body: {
      rating: number;
      comment?: string;
      semester?: string;
      score?: string;
      captchaToken: string;
    },
    idempotencyKey: string,
  ) {
    return apiRequest<Review>(`/courses/${encodeURIComponent(id)}/reviews`, {
      method: "POST",
      body,
      headers: { "Idempotency-Key": idempotencyKey },
    });
  },

  likeReview(id: string) {
    return apiRequest<void>(`/reviews/${encodeURIComponent(id)}/like`, { method: "POST" });
  },

  unlikeReview(id: string) {
    return apiRequest<void>(`/reviews/${encodeURIComponent(id)}/like`, { method: "DELETE" });
  },

  reportReview(id: string, reason: string, captchaToken: string) {
    return apiRequest<void>(`/reviews/${encodeURIComponent(id)}/report`, {
      method: "POST",
      body: { reason, captchaToken },
    });
  },

  search(
    q: string,
    type: "course" | "teacher" | "review" | "thread" | "user" | "board" | "tag" | "all" = "all",
    limit = 12,
    cursor?: string | null,
  ) {
    return apiRequest<SearchResult>("/search", {
      query: { q, type, limit, cursor },
      auth: "optional",
    });
  },

  calendars() {
    return apiRequest<Calendar[]>("/selection/calendars", { auth: false });
  },

  campuses() {
    return apiRequest<Campus[]>("/selection/campuses", { auth: false });
  },

  faculties() {
    return apiRequest<Faculty[]>("/selection/faculties", { auth: false });
  },

  grades(calendarId: string) {
    return apiRequest<string[]>("/selection/grades", {
      query: { calendarId },
      auth: false,
    });
  },

  majors(grade: string) {
    return apiRequest<Major[]>("/selection/majors", {
      query: { grade },
      auth: false,
    });
  },

  courseNatures() {
    return apiRequest<CourseNature[]>("/selection/course-natures", { auth: false });
  },

  selectionByMajor(majorId: string, grade: string) {
    return apiRequest<SelectionCourse[]>("/selection/courses-by-major", {
      query: { majorId, grade },
      auth: false,
    });
  },

  selectionByNature(natureId: string) {
    return apiRequest<SelectionCourse[]>("/selection/courses-by-nature", {
      query: { natureId },
      auth: false,
    });
  },

  selectionSearch(q: string) {
    return apiRequest<SelectionCourse[]>("/selection/courses/search", {
      query: { q },
      auth: false,
    });
  },

  selectionCourse(code: string) {
    return apiRequest<SelectionCourse>(`/selection/courses/${encodeURIComponent(code)}`, {
      auth: false,
    });
  },

  selectionTimeslots(code: string) {
    return apiRequest<TimeSlot[]>(`/selection/courses/${encodeURIComponent(code)}/timeslots`, {
      auth: false,
    });
  },

  selectionLatestUpdate() {
    return apiRequest<LatestUpdate>("/selection/latest-update", { auth: false });
  },

  boards() {
    return apiRequest<Board[]>("/forum/boards");
  },

  tags() {
    return apiRequest<Tag[]>("/forum/tags", { auth: false });
  },

  threads(query: {
    board?: string;
    tag?: string;
    feed?: "hot" | "new" | "subscriptions" | "following" | "unread";
    cursor?: string | null;
  }) {
    return apiRequest<Page<ThreadFeed>>("/forum/threads", {
      query: { board: query.board, tag: query.tag, sort: query.feed, cursor: query.cursor, limit: 20 },
    });
  },

  createThread(body: {
    boardId: string;
    title: string;
    body?: string;
    contentFormat?: ContentFormat;
    tags?: string[];
    poll?: { question: string; multiSelect?: boolean; options: string[] };
  }) {
    return apiRequest<ThreadDetailWithPoll>("/forum/threads", { method: "POST", body });
  },

  thread(id: string) {
    return apiRequest<ThreadDetailWithPoll>(`/forum/threads/${encodeURIComponent(id)}`);
  },

  updateThread(id: string, body: {
    expectedVersion: number;
    title?: string;
    body?: string;
    contentFormat?: ContentFormat;
    tags?: string[];
  }) {
    return apiRequest<ThreadDetailWithPoll>(`/forum/threads/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  deleteThread(id: string) {
    return apiRequest<{ ok: boolean }>(`/forum/threads/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
  },

  comments(threadId: string, cursor?: string | null) {
    return apiRequest<Page<Comment>>(`/forum/threads/${encodeURIComponent(threadId)}/comments`, {
      query: { cursor, limit: 50 },
    });
  },

  addComment(
    threadId: string,
    body: string,
    contentFormat: ContentFormat = "markdown_v1",
    parentId?: string,
  ) {
    return apiRequest<Comment>(`/forum/threads/${encodeURIComponent(threadId)}/comments`, {
      method: "POST",
      body: { body, contentFormat, parentId },
    });
  },

  updateComment(id: string, body: {
    expectedVersion: number;
    body: string;
    contentFormat: ContentFormat;
  }) {
    return apiRequest<Comment>(`/forum/comments/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  deleteComment(id: string) {
    return apiRequest<{ ok: boolean }>(`/forum/comments/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
  },

  moderateForumThread(
    id: string,
    action:
      | "pin"
      | "unpin"
      | "close"
      | "reopen"
      | "archive"
      | "unarchive"
      | "delete"
      | "restore"
      | "hide"
      | "unhide"
      | "move",
    body: { reason: string; globally?: boolean; boardId?: string },
  ) {
    return apiRequest<{ ok: boolean }>(
      `/admin/forum/threads/${encodeURIComponent(id)}/${action}`,
      { method: "POST", body },
    );
  },

  moderateForumComment(
    id: string,
    action: "delete" | "restore" | "hide" | "unhide",
    reason: string,
  ) {
    return apiRequest<{ ok: boolean }>(
      `/admin/forum/comments/${encodeURIComponent(id)}/${action}`,
      { method: "POST", body: { reason } },
    );
  },

  votePost(id: string, value: "up" | "down", postType: "thread" | "comment" = "thread") {
    return apiRequest<{ ok: boolean; voteCount: number; viewerVote: "up" | "down" | null }>(`/forum/posts/${encodeURIComponent(id)}/vote`, {
      method: "POST",
      body: { value, postType },
    });
  },

  removePostVote(id: string, postType: "thread" | "comment" = "thread") {
    return apiRequest<{ ok: boolean; voteCount: number; viewerVote: null }>(
      `/forum/posts/${encodeURIComponent(id)}/vote`,
      { method: "DELETE", query: { postType } },
    );
  },

  flagPost(
    id: string,
    reason: "spam" | "abuse" | "off_topic" | "illegal" | "other",
    note?: string,
    postType: "thread" | "comment" = "thread",
  ) {
    return apiRequest<{ ok: boolean; autoHidden: boolean; autoSilenced: boolean }>(`/forum/posts/${encodeURIComponent(id)}/flag`, {
      method: "POST",
      body: { reason, note, postType },
    });
  },

  bookmarkPost(id: string, postType: "thread" | "comment" = "thread", note?: string) {
    return apiRequest<void>(`/forum/posts/${encodeURIComponent(id)}/bookmark`, {
      method: "PUT",
      body: { postType, note },
    });
  },

  removeBookmark(id: string, postType: "thread" | "comment" = "thread") {
    return apiRequest<void>(`/forum/posts/${encodeURIComponent(id)}/bookmark`, {
      method: "DELETE",
      query: { postType },
    });
  },

  bookmarks(cursor?: string | null) {
    return apiRequest<Page<Bookmark>>("/forum/bookmarks", { query: { cursor, limit: 30 } });
  },

  subscriptions(cursor?: string | null, targetType?: "board" | "thread") {
    return apiRequest<Page<{ targetType: "board" | "thread"; targetId: string; level: string; createdAt: number }>>(
      "/forum/subscriptions",
      { query: { cursor, type: targetType, limit: 30 } },
    );
  },

  setSubscription(body: { targetType: "board" | "thread"; targetId: string; level: string }) {
    return apiRequest<void>("/forum/subscriptions", { method: "PUT", body });
  },

  deleteSubscription(body: { targetType: "board" | "thread"; targetId: string }) {
    return apiRequest<void>("/forum/subscriptions", { method: "DELETE", body });
  },

  reportThreadRead(id: string, lastReadCommentId?: string | null) {
    return apiRequest<void>(`/forum/threads/${encodeURIComponent(id)}/read`, {
      method: "POST",
      body: { lastReadCommentId },
    });
  },

  ignoredUsers(cursor?: string | null) {
    return apiRequest<Page<IgnoreUser>>("/me/ignores", { query: { cursor, limit: 100 } });
  },

  ignoreUser(accountId: string) {
    return apiRequest<void>(`/me/ignores/${encodeURIComponent(accountId)}`, { method: "PUT" });
  },

  unignoreUser(accountId: string) {
    return apiRequest<void>(`/me/ignores/${encodeURIComponent(accountId)}`, {
      method: "DELETE",
    });
  },

  dmConversations(query: {
    cursor?: string | null;
    view?: "inbox" | "requests" | "sent" | "archived" | "deleted";
    q?: string;
  } = {}) {
    return apiRequest<Page<DmConversation>>("/forum/dm/conversations", {
      query: { ...query, limit: 30 },
    });
  },

  dmUnreadCount() {
    return apiRequest<DmCounts>("/forum/dm/unread-count");
  },

  createDmConversation(recipientHandle: string, requestMessage: string, idempotencyKey: string) {
    return apiRequest<DmConversation>("/forum/dm/conversations", {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: { recipientHandle, requestMessage },
    });
  },

  acceptDmRequest(id: string) {
    return apiRequest<DmConversation>(`/forum/dm/requests/${encodeURIComponent(id)}/accept`, {
      method: "POST",
    });
  },

  declineDmRequest(id: string) {
    return apiRequest<void>(`/forum/dm/requests/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
  },

  reportDmRequest(id: string, reason: DmReportReason, note?: string) {
    return apiRequest<void>(`/forum/dm/requests/${encodeURIComponent(id)}/report`, {
      method: "POST",
      body: { reason, note },
    });
  },

  dmMessages(id: string, cursor?: string | null) {
    return apiRequest<Page<DmMessage>>(`/forum/dm/conversations/${encodeURIComponent(id)}/messages`, {
      query: { cursor, limit: 50 },
    });
  },

  sendDmMessage(id: string, body: string) {
    return apiRequest<DmMessage>(`/forum/dm/conversations/${encodeURIComponent(id)}/messages`, {
      method: "POST",
      body: { body },
    });
  },

  markDmConversationRead(id: string, lastReadMessageId?: string | null) {
    return apiRequest<void>(`/forum/dm/conversations/${encodeURIComponent(id)}/read`, {
      method: "POST",
      body: { lastReadMessageId },
    });
  },

  setDmConversationArchived(id: string, isArchived: boolean) {
    return apiRequest<void>(`/forum/dm/conversations/${encodeURIComponent(id)}/archive`, {
      method: isArchived ? "PUT" : "DELETE",
    });
  },

  setDmConversationMuted(id: string, isMuted: boolean) {
    return apiRequest<void>(`/forum/dm/conversations/${encodeURIComponent(id)}/mute`, {
      method: isMuted ? "PUT" : "DELETE",
    });
  },

  deleteDmConversation(id: string) {
    return apiRequest<void>(`/forum/dm/conversations/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
  },

  recoverDmConversation(id: string) {
    return apiRequest<void>(`/forum/dm/conversations/${encodeURIComponent(id)}/recover`, {
      method: "POST",
    });
  },

  reportDmMessage(id: string, reason: DmReportReason, note?: string) {
    return apiRequest<void>(`/forum/dm/messages/${encodeURIComponent(id)}/report`, {
      method: "POST",
      body: { reason, note },
    });
  },

  votePoll(id: string, optionId: string) {
    return apiRequest<{ ok: boolean; myVotes: string[] }>(`/forum/polls/${encodeURIComponent(id)}/vote`, {
      method: "POST",
      body: { optionId },
    });
  },

  removePollVote(id: string, optionId: string) {
    return apiRequest<{ ok: boolean; myVotes: string[] }>(
      `/forum/polls/${encodeURIComponent(id)}/vote`,
      { method: "DELETE", query: { optionId } },
    );
  },

  pollResults(id: string) {
    return apiRequest<Poll>(
      `/forum/polls/${encodeURIComponent(id)}/results`,
      { auth: false },
    );
  },

  notifications(unread?: boolean, cursor?: string | null) {
    return apiRequest<Page<Notification>>("/notifications", {
      query: { unread, cursor, limit: 30 },
    });
  },

  unreadNotificationCount() {
    return apiRequest<{ count?: number }>("/notifications/unread-count");
  },

  markNotificationsRead(ids?: string[]) {
    return apiRequest<void>("/notifications/read", {
      method: "POST",
      body: ids ? { ids } : { all: true },
    });
  },

  notificationPrefs() {
    return apiRequest<{ prefs: NotificationPreferences }>("/me/notification-prefs");
  },

  updateNotificationPrefs(prefs: NotificationPreferences) {
    return apiRequest<{ prefs: NotificationPreferences }>("/me/notification-prefs", {
      method: "PUT",
      body: { prefs },
    });
  },

  wallet() {
    return apiRequest<Wallet>("/wallet");
  },

  bindWallet(publicKey: string) {
    return apiRequest<void>("/wallet/bind", { method: "POST", body: { publicKey } });
  },

  claimChallenge() {
    return apiRequest<{ challengeId?: string; nonce?: string }>("/wallet/claim-challenge");
  },

  claimWallet(body: { legacyUserHash: string; challengeId: string; signature: string }) {
    return apiRequest<Wallet>("/wallet/claim", { method: "POST", body });
  },

  ledger(cursor?: string | null) {
    return apiRequest<Page<LedgerEntry>>("/wallet/ledger", {
      query: { cursor, limit: 30 },
    });
  },

  verifyLedger() {
    return apiRequest<LedgerVerify>("/wallet/ledger/verify", { auth: false });
  },

  adminCreditReconciliationStats() {
    return apiRequest<CreditReconciliationStats>("/admin/credit/reconciliations/stats");
  },

  adminCreditReconciliations(cursor?: string | null) {
    return apiRequest<Page<CreditReconciliationRun>>("/admin/credit/reconciliations", {
      query: { cursor, limit: 30 },
    });
  },

  requestAdminCreditReconciliation(reason: string, idempotencyKey: string) {
    return apiRequest<CreditReconciliationRun>("/admin/credit/reconciliations", {
      method: "POST",
      body: { reason },
      headers: { "Idempotency-Key": idempotencyKey },
    });
  },

  adminCreditReconciliation(id: string) {
    return apiRequest<CreditReconciliationRun>(
      `/admin/credit/reconciliations/${encodeURIComponent(id)}`,
    );
  },

  resumeAdminCreditReconciliation(id: string, reason: string) {
    return apiRequest<CreditReconciliationRun>(
      `/admin/credit/reconciliations/${encodeURIComponent(id)}/resume`,
      { method: "POST", body: { reason } },
    );
  },

  adminCreditReconciliationWallets(
    id: string,
    cursor?: string | null,
    driftOnly = true,
  ) {
    return apiRequest<Page<CreditReconciliationWallet>>(
      `/admin/credit/reconciliations/${encodeURIComponent(id)}/wallets`,
      { query: { cursor, driftOnly, limit: 50 } },
    );
  },

  creditSigningIntent(
    action:
      | "credit.tip"
      | "credit.task.create"
      | "credit.task.action"
      | "credit.product.purchase"
      | "credit.purchase.action",
    request: Record<string, unknown>,
    idempotencyKey: string,
  ) {
    return apiRequest<SigningIntent>("/credit/signing-intents", {
      method: "POST",
      body: { action, request },
      headers: { "Idempotency-Key": idempotencyKey },
    });
  },

  tip(
    body: { toAccountId: string; amount: number; targetType: "review" | "thread" | "comment"; targetId: string },
    authorization: WalletAuthorization,
  ) {
    return apiRequest<void>("/credit/tip", {
      method: "POST",
      body,
      headers: walletHeaders(authorization),
    });
  },

  tasks(status?: string, cursor?: string | null) {
    return apiRequest<Page<Task>>("/credit/tasks", {
      query: { status, cursor, limit: 30 },
    });
  },

  createTask(
    body: { title: string; description?: string; rewardAmount: number; contactInfo?: string },
    authorization: WalletAuthorization,
  ) {
    return apiRequest<Task>("/credit/tasks", {
      method: "POST",
      body,
      headers: walletHeaders(authorization),
    });
  },

  acceptTask(id: string) {
    return apiRequest<void>(`/credit/tasks/${encodeURIComponent(id)}/accept`, { method: "POST" });
  },

  taskAction(
    id: string,
    action: "submit" | "confirm" | "cancel" | "reject" | "delete",
    authorization?: WalletAuthorization,
  ) {
    return apiRequest<void>(`/credit/tasks/${encodeURIComponent(id)}/action`, {
      method: "POST",
      body: { action },
      headers: authorization ? walletHeaders(authorization) : undefined,
    });
  },

  products(cursor?: string | null) {
    return apiRequest<Page<Product>>("/credit/products", { query: { cursor, limit: 30 } });
  },

  createProduct(body: { title: string; description?: string; price: number; stock: number; deliveryInfo?: string }) {
    return apiRequest<Product>("/credit/products", { method: "POST", body });
  },

  purchaseProduct(id: string, authorization: WalletAuthorization) {
    return apiRequest<Purchase>(`/credit/products/${encodeURIComponent(id)}/purchase`, {
      method: "POST",
      headers: walletHeaders(authorization),
    });
  },

  purchases(cursor?: string | null) {
    return apiRequest<Page<Purchase>>("/credit/purchases", { query: { cursor, limit: 30 } });
  },

  purchaseAction(
    id: string,
    action: "accept" | "deliver" | "confirm" | "cancel",
    authorization?: WalletAuthorization,
  ) {
    return apiRequest<void>(`/credit/purchases/${encodeURIComponent(id)}/action`, {
      method: "POST",
      body: { action },
      headers: authorization ? walletHeaders(authorization) : undefined,
    });
  },

  adminOverview() {
    return apiRequest<AdminOverview>("/admin/overview");
  },

  adminAuditEvents(query: {
    actorId?: string;
    action?: string;
    targetType?: string;
    cursor?: string | null;
  }) {
    return apiRequest<Page<AdminAuditEvent>>("/admin/audit-events", {
      query: { ...query, limit: 30 },
    });
  },

  adminActivityPolicy() {
    return apiRequest<ActivityPolicy>("/admin/activity-policy");
  },

  updateAdminActivityPolicy(body: ActivityPolicyUpdateInput) {
    return apiRequest<ActivityPolicy>("/admin/activity-policy", { method: "PUT", body });
  },

  adminActivityPolicyHistory(cursor?: string | null) {
    return apiRequest<Page<ActivityPolicy>>("/admin/activity-policy/history", {
      query: { cursor, limit: 30 },
    });
  },

  adminUsers(query: {
    q?: string;
    role?: "user" | "mod" | "admin";
    status?: "active" | "suspended" | "deleted";
    cursor?: string | null;
  }) {
    return apiRequest<Page<AdminUser>>("/admin/users", { query: { ...query, limit: 30 } });
  },

  inviteAdminUser(body: AdminUserInviteInput) {
    return apiRequest<AdminUser>("/admin/users", { method: "POST", body });
  },

  updateAdminUserRole(id: string, role: "user" | "mod", reason: string) {
    return apiRequest<AdminUser>(`/admin/users/${encodeURIComponent(id)}/role`, {
      method: "PATCH",
      body: { role, reason },
    });
  },

  revokeAdminUserSessions(id: string, reason: string) {
    return apiRequest<void>(`/admin/users/${encodeURIComponent(id)}/sessions/revoke`, {
      method: "POST",
      body: { reason },
    });
  },

  adminUserSanctions(id: string) {
    return apiRequest<Sanction[]>(`/admin/users/${encodeURIComponent(id)}/sanctions`);
  },

  unsanctionAdminUser(id: string, sanctionId: string, reason: string) {
    return apiRequest<void>(`/admin/users/${encodeURIComponent(id)}/unsanction`, {
      method: "POST",
      body: { sanctionId, reason },
    });
  },

  sanctionAdminUser(
    id: string,
    kind: "silence" | "suspend",
    body: { reason: string; endsAt?: number | null },
  ) {
    return apiRequest<void>(`/admin/users/${encodeURIComponent(id)}/${kind}`, {
      method: "POST",
      body,
    });
  },

  adminReviews(status: "visible" | "hidden" | "pending" | "all" = "pending", cursor?: string | null) {
    return apiRequest<Page<Review>>("/admin/reviews", { query: { status, cursor, limit: 30 } });
  },

  toggleReview(id: string, reason: string) {
    return apiRequest<{ ok: boolean }>(`/admin/reviews/${encodeURIComponent(id)}/toggle`, {
      method: "POST",
      body: { reason },
    });
  },

  deleteAdminReview(id: string, reason: string) {
    return apiRequest<void>(`/admin/reviews/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: { reason },
    });
  },

  adminReports(
    status: "open" | "upheld" | "rejected" | "ignored" | "all" = "open",
    cursor?: string | null,
  ) {
    return apiRequest<Page<ReviewReport>>("/admin/reports", {
      query: { status, cursor, limit: 30 },
    });
  },

  resolveReport(id: string, action: "uphold" | "reject" | "ignore", note: string) {
    return apiRequest<ReviewReport>(`/admin/reports/${encodeURIComponent(id)}/resolve`, {
      method: "POST",
      body: { action, note },
    });
  },

  adminForumFlags(status: "open" | "all" = "open", cursor?: string | null) {
    return apiRequest<Page<AdminForumFlag>>("/admin/forum/flags", {
      query: { status, cursor, limit: 30 },
    });
  },

  adminForumThread(id: string) {
    return apiRequest<ThreadDetailWithPoll>(`/admin/forum/threads/${encodeURIComponent(id)}`);
  },

  adminForumComment(id: string) {
    return apiRequest<Comment>(`/admin/forum/comments/${encodeURIComponent(id)}`);
  },

  resolveAdminForumFlag(id: string, action: "uphold" | "reject" | "ignore", note: string) {
    return apiRequest<AdminForumFlag>(`/admin/forum/flags/${encodeURIComponent(id)}/resolve`, {
      method: "POST",
      body: { action, note },
    });
  },

  adminDmReports(status: "open" | "upheld" | "rejected" = "open", cursor?: string | null) {
    return apiRequest<Page<DmReport>>("/admin/dm/reports", {
      query: { status, cursor, limit: 30 },
    });
  },

  resolveAdminDmReport(id: string, action: "uphold" | "reject", note?: string) {
    return apiRequest<DmReport>(`/admin/dm/reports/${encodeURIComponent(id)}/resolve`, {
      method: "POST",
      body: { action, note },
    });
  },

  adminSettings() {
    return apiRequest<Setting[]>("/admin/settings");
  },

  updateAdminSetting(key: string, body: SettingUpdateInput) {
    return apiRequest<Setting>(`/admin/settings/${encodeURIComponent(key)}`, {
      method: "PUT",
      body,
    });
  },

  adminVerificationTypes(cursor?: string | null) {
    return apiRequest<Page<VerificationType>>("/admin/verifications/types", {
      query: { cursor, limit: 50 },
    });
  },

  createAdminVerificationType(body: VerificationTypeInput) {
    return apiRequest<VerificationType>("/admin/verifications/types", { method: "POST", body });
  },

  adminUserVerifications(accountId: string, cursor?: string | null) {
    return apiRequest<Page<VerificationGrant>>(
      `/admin/users/${encodeURIComponent(accountId)}/verifications`,
      { query: { cursor, limit: 50 } },
    );
  },

  grantAdminUserVerification(accountId: string, body: VerificationGrantInput) {
    return apiRequest<VerificationGrant>(
      `/admin/users/${encodeURIComponent(accountId)}/verifications`,
      { method: "POST", body },
    );
  },

  revokeAdminUserVerification(grantId: string, reason: string) {
    return apiRequest<VerificationGrant>(
      `/admin/verifications/grants/${encodeURIComponent(grantId)}/revoke`,
      { method: "POST", body: { reason } },
    );
  },

  adminAchievements(cursor?: string | null) {
    return apiRequest<Page<Achievement>>("/admin/achievements", {
      query: { cursor, limit: 50 },
    });
  },

  createAdminAchievement(body: AchievementCreateInput) {
    return apiRequest<Achievement>("/admin/achievements", { method: "POST", body });
  },

  updateAdminAchievement(id: string, body: AchievementUpdateInput) {
    return apiRequest<Achievement>(`/admin/achievements/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  adminUserAchievements(accountId: string, cursor?: string | null) {
    return apiRequest<Page<AchievementGrant>>(
      `/admin/users/${encodeURIComponent(accountId)}/achievements`,
      { query: { cursor, limit: 50 } },
    );
  },

  grantAdminUserAchievement(accountId: string, body: AchievementGrantInput) {
    return apiRequest<AchievementGrant>(
      `/admin/users/${encodeURIComponent(accountId)}/achievements`,
      { method: "POST", body },
    );
  },

  revokeAdminUserAchievement(accountId: string, achievementId: string, reason: string) {
    return apiRequest<AchievementGrant>(
      `/admin/users/${encodeURIComponent(accountId)}/achievements/${encodeURIComponent(achievementId)}/revoke`,
      { method: "POST", body: { reason } },
    );
  },

  adminUserAchievementEvents(accountId: string, cursor?: string | null) {
    return apiRequest<Page<AchievementEvent>>(
      `/admin/users/${encodeURIComponent(accountId)}/achievement-events`,
      { query: { cursor, limit: 50 } },
    );
  },

  adminAnnouncements(cursor?: string | null) {
    return apiRequest<Page<Announcement>>("/admin/announcements", {
      query: { cursor, limit: 30 },
    });
  },

  createAdminAnnouncement(body: AnnouncementCreateInput) {
    return apiRequest<Announcement>("/admin/announcements", { method: "POST", body });
  },

  updateAdminAnnouncement(id: string, body: AnnouncementUpdateInput) {
    return apiRequest<Announcement>(`/admin/announcements/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  archiveAdminAnnouncement(id: string, body: AdminVersionedArchiveInput) {
    return apiRequest<void>(`/admin/announcements/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body,
    });
  },

  adminAnnouncementRevisions(id: string, cursor?: string | null) {
    return apiRequest<Page<AnnouncementRevision>>(
      `/admin/announcements/${encodeURIComponent(id)}/revisions`,
      { query: { cursor, limit: 30 } },
    );
  },

  adminPromotions(cursor?: string | null) {
    return apiRequest<Page<Promotion>>("/admin/promotions", { query: { cursor, limit: 30 } });
  },

  createAdminPromotion(body: PromotionCreateInput) {
    return apiRequest<Promotion>("/admin/promotions", { method: "POST", body });
  },

  updateAdminPromotion(id: string, body: PromotionUpdateInput) {
    return apiRequest<Promotion>(`/admin/promotions/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  archiveAdminPromotion(id: string, body: AdminVersionedArchiveInput) {
    return apiRequest<void>(`/admin/promotions/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body,
    });
  },

  adminPromotionMetrics(id: string, from?: string, to?: string) {
    return apiRequest<PromotionMetrics>(`/admin/promotions/${encodeURIComponent(id)}/metrics`, {
      query: { from, to },
    });
  },

  adminCourses(cursor?: string | null) {
    return apiRequest<Page<Course>>("/admin/courses", { query: { cursor, limit: 30 } });
  },

  createAdminCourse(body: AdminCourseCreateInput) {
    return apiRequest<Course>("/admin/courses", { method: "POST", body });
  },

  updateAdminCourse(id: string, body: AdminCourseUpdateInput) {
    return apiRequest<Course>(`/admin/courses/${encodeURIComponent(id)}`, {
      method: "PUT",
      body,
    });
  },

  deleteAdminCourse(id: string, reason: string) {
    return apiRequest<void>(`/admin/courses/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: { reason },
    });
  },

  adminMediaUploads(cursor?: string | null) {
    return apiRequest<Page<Upload>>("/admin/media/uploads", { query: { cursor, limit: 30 } });
  },

  moderateAdminMediaUpload(id: string, action: "approve" | "block", reason: string) {
    return apiRequest<{ ok: boolean }>(`/admin/media/uploads/${encodeURIComponent(id)}/${action}`, {
      method: "POST",
      body: { reason },
    });
  },

  createAdminBoard(body: AdminBoardCreateInput) {
    return apiRequest<Board>("/admin/forum/boards", { method: "POST", body });
  },

  updateAdminBoard(id: string, body: AdminBoardUpdateInput) {
    return apiRequest<Board>(`/admin/forum/boards/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  deleteAdminBoard(id: string, reason: string) {
    return apiRequest<{ ok: boolean }>(`/admin/forum/boards/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: { reason },
    });
  },

  adminTags() {
    return apiRequest<Tag[]>("/admin/forum/tags");
  },

  createAdminTag(body: AdminTagCreateInput) {
    return apiRequest<Tag>("/admin/forum/tags", { method: "POST", body });
  },

  updateAdminTag(id: string, body: AdminTagUpdateInput) {
    return apiRequest<Tag>(`/admin/forum/tags/${encodeURIComponent(id)}`, {
      method: "PATCH",
      body,
    });
  },

  deleteAdminTag(id: string, reason: string) {
    return apiRequest<{ ok: boolean }>(`/admin/forum/tags/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: { reason },
    });
  },

  adminWatchedWords() {
    return apiRequest<WatchedWord[]>("/admin/forum/watched-words");
  },

  createAdminWatchedWord(body: WatchedWordInput) {
    return apiRequest<WatchedWord>("/admin/forum/watched-words", { method: "POST", body });
  },

  deleteAdminWatchedWord(id: string, reason: string) {
    return apiRequest<{ ok: boolean }>(`/admin/forum/watched-words/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: { reason },
    });
  },

  triggerSelectionSync(reason: string) {
    return apiRequest<void>("/admin/selection/sync", { method: "POST", body: { reason } });
  },

  reindexCourses(reason: string) {
    return apiRequest<void>("/admin/courses/reindex", { method: "POST", body: { reason } });
  },

  reindexReviews(reason: string) {
    return apiRequest<void>("/admin/reviews/reindex", { method: "POST", body: { reason } });
  },

  reindexForum(reason: string) {
    return apiRequest<void>("/admin/forum/reindex", { method: "POST", body: { reason } });
  },
};

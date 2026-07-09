import { apiRequest } from "./client";
import type {
  Account,
  Announcement,
  Board,
  Bookmark,
  Calendar,
  Campus,
  Comment,
  Course,
  CourseDetail,
  CourseNature,
  Department,
  DmConversation,
  DmMessage,
  Faculty,
  LatestUpdate,
  LedgerEntry,
  LedgerVerify,
  Major,
  Notification,
  Page,
  Product,
  Purchase,
  Review,
  SearchResult,
  SelectionCourse,
  Setting,
  Tag,
  Task,
  ThreadDetailWithPoll,
  ThreadFeed,
  TimeSlot,
  Wallet,
} from "./types";

export const api = {
  requestEmailCode(email: string) {
    return apiRequest<void>("/auth/email/request-code", {
      method: "POST",
      body: { email },
      auth: false,
    });
  },

  verifyEmail(input: { email: string; code: string; handle?: string; password?: string }) {
    return apiRequest<{ accessToken: string; refreshToken: string; account: Account }>(
      "/auth/email/verify",
      { method: "POST", body: input, auth: false },
    );
  },

  logout() {
    return apiRequest<void>("/auth/logout", { method: "POST" });
  },

  me() {
    return apiRequest<Account>("/me");
  },

  updateMe(input: { handle?: string; avatarUrl?: string }) {
    return apiRequest<Account>("/me", { method: "PATCH", body: input });
  },

  publicUser(handle: string) {
    return apiRequest<Account & { threadCount?: number; commentCount?: number }>(
      `/users/${encodeURIComponent(handle)}`,
      { auth: false },
    );
  },

  userThreads(handle: string, cursor?: string | null) {
    return apiRequest<Page<ThreadFeed>>(`/users/${encodeURIComponent(handle)}/threads`, {
      query: { cursor, limit: 20 },
      auth: false,
    });
  },

  userComments(handle: string, cursor?: string | null) {
    return apiRequest<Page<Comment>>(`/users/${encodeURIComponent(handle)}/comments`, {
      query: { cursor, limit: 20 },
      auth: false,
    });
  },

  announcements() {
    return apiRequest<Announcement[]>("/announcements", { auth: false });
  },

  settings() {
    return apiRequest<Setting[]>("/settings", { auth: false });
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
    body: { rating: number; comment?: string; semester?: string; score?: string },
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

  reportReview(id: string, reason: string) {
    return apiRequest<void>(`/reviews/${encodeURIComponent(id)}/report`, {
      method: "POST",
      body: { reason },
    });
  },

  search(q: string, type: "course" | "teacher" | "review" | "all" = "all") {
    return apiRequest<SearchResult>("/search", {
      query: { q, type, limit: 12 },
      auth: false,
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
    return apiRequest<Board[]>("/forum/boards", { auth: false });
  },

  tags() {
    return apiRequest<Tag[]>("/forum/tags", { auth: false });
  },

  threads(query: {
    board?: string;
    tag?: string;
    feed?: "hot" | "new" | "following" | "unread";
    cursor?: string | null;
  }) {
    return apiRequest<Page<ThreadFeed>>("/forum/threads", {
      query: { board: query.board, tag: query.tag, sort: query.feed, cursor: query.cursor, limit: 20 },
      auth: false,
    });
  },

  createThread(body: {
    boardId: string;
    title: string;
    body?: string;
    tags?: string[];
    poll?: { question: string; multiSelect?: boolean; options: string[] };
  }) {
    return apiRequest<ThreadDetailWithPoll>("/forum/threads", { method: "POST", body });
  },

  thread(id: string) {
    return apiRequest<ThreadDetailWithPoll>(`/forum/threads/${encodeURIComponent(id)}`, { auth: false });
  },

  comments(threadId: string, cursor?: string | null) {
    return apiRequest<Page<Comment>>(`/forum/threads/${encodeURIComponent(threadId)}/comments`, {
      query: { cursor, limit: 50 },
      auth: false,
    });
  },

  addComment(threadId: string, body: string, parentId?: string) {
    return apiRequest<Comment>(`/forum/threads/${encodeURIComponent(threadId)}/comments`, {
      method: "POST",
      body: { body, parentId },
    });
  },

  votePost(id: string, value: "up" | "down", postType: "thread" | "comment" = "thread") {
    return apiRequest<void>(`/forum/posts/${encodeURIComponent(id)}/vote`, {
      method: "POST",
      body: { value, postType },
    });
  },

  flagPost(
    id: string,
    reason: "spam" | "abuse" | "off_topic" | "illegal" | "other",
    note?: string,
    postType: "thread" | "comment" = "thread",
  ) {
    return apiRequest<void>(`/forum/posts/${encodeURIComponent(id)}/flag`, {
      method: "POST",
      body: { reason, note, postType },
    });
  },

  bookmarkPost(id: string, note?: string) {
    return apiRequest<void>(`/forum/posts/${encodeURIComponent(id)}/bookmark`, {
      method: "PUT",
      body: { note },
    });
  },

  bookmarks(cursor?: string | null) {
    return apiRequest<Page<Bookmark>>("/forum/bookmarks", { query: { cursor, limit: 30 } });
  },

  subscriptions() {
    return apiRequest<Array<{ targetType?: string; targetId?: string; level?: string }>>(
      "/forum/subscriptions",
    );
  },

  setSubscription(body: { targetType: "board" | "thread"; targetId: string; level: string }) {
    return apiRequest<void>("/forum/subscriptions", { method: "PUT", body });
  },

  dmConversations() {
    return apiRequest<DmConversation[]>("/forum/dm/conversations");
  },

  createDmConversation(recipientId: string) {
    return apiRequest<{ id?: string }>("/forum/dm/conversations", {
      method: "POST",
      body: { recipientId },
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

  votePoll(id: string, optionId: string) {
    return apiRequest<void>(`/forum/polls/${encodeURIComponent(id)}/vote`, {
      method: "POST",
      body: { optionId },
    });
  },

  pollResults(id: string) {
    return apiRequest<{ id?: string; question?: string; options?: Array<{ id?: string; label?: string; voteCount?: number }>; myVotes?: string[] }>(
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
    return apiRequest<void>("/notifications/read", { method: "POST", body: { ids } });
  },

  notificationPrefs() {
    return apiRequest<{ prefs?: Record<string, unknown> }>("/me/notification-prefs");
  },

  updateNotificationPrefs(prefs: Record<string, unknown>) {
    return apiRequest<{ prefs?: Record<string, unknown> }>("/me/notification-prefs", {
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

  tip(
    body: { toAccountId: string; amount: number; targetType: "review" | "thread" | "comment"; targetId: string },
    walletSig: string,
    idempotencyKey: string,
  ) {
    return apiRequest<void>("/credit/tip", {
      method: "POST",
      body,
      headers: { "X-Wallet-Sig": walletSig, "Idempotency-Key": idempotencyKey },
    });
  },

  tasks(status?: string, cursor?: string | null) {
    return apiRequest<Page<Task>>("/credit/tasks", {
      query: { status, cursor, limit: 30 },
    });
  },

  createTask(body: { title: string; description?: string; rewardAmount: number; contactInfo?: string }, walletSig: string) {
    return apiRequest<Task>("/credit/tasks", {
      method: "POST",
      body,
      headers: { "X-Wallet-Sig": walletSig },
    });
  },

  acceptTask(id: string) {
    return apiRequest<Task>(`/credit/tasks/${encodeURIComponent(id)}/accept`, { method: "POST" });
  },

  taskAction(id: string, action: "submit" | "confirm" | "cancel" | "reject" | "delete", walletSig: string) {
    return apiRequest<Task>(`/credit/tasks/${encodeURIComponent(id)}/action`, {
      method: "POST",
      body: { action },
      headers: { "X-Wallet-Sig": walletSig },
    });
  },

  products(cursor?: string | null) {
    return apiRequest<Page<Product>>("/credit/products", { query: { cursor, limit: 30 } });
  },

  createProduct(body: { title: string; description?: string; price: number; stock: number; deliveryInfo?: string }) {
    return apiRequest<Product>("/credit/products", { method: "POST", body });
  },

  purchaseProduct(id: string, walletSig: string) {
    return apiRequest<Purchase>(`/credit/products/${encodeURIComponent(id)}/purchase`, {
      method: "POST",
      headers: { "X-Wallet-Sig": walletSig },
    });
  },

  purchases(cursor?: string | null) {
    return apiRequest<Page<Purchase>>("/credit/purchases", { query: { cursor, limit: 30 } });
  },

  purchaseAction(id: string, action: "accept" | "deliver" | "confirm", walletSig: string) {
    return apiRequest<Purchase>(`/credit/purchases/${encodeURIComponent(id)}/action`, {
      method: "POST",
      body: { action },
      headers: { "X-Wallet-Sig": walletSig },
    });
  },

  adminReviews(status: "visible" | "hidden" | "pending" | "all" = "pending") {
    return apiRequest<Page<Review>>("/admin/reviews", { query: { status, limit: 30 } });
  },

  toggleReview(id: string) {
    return apiRequest<Review>(`/admin/reviews/${encodeURIComponent(id)}/toggle`, { method: "POST" });
  },

  adminReports(status: "open" | "resolved" | "all" = "open") {
    return apiRequest<Page<{ id?: string; reviewId?: string; reason?: string; status?: string; createdAt?: number }>>(
      "/admin/reports",
      { query: { status, limit: 30 } },
    );
  },

  resolveReport(id: string, action: string, note?: string) {
    return apiRequest<void>(`/admin/reports/${encodeURIComponent(id)}/resolve`, {
      method: "POST",
      body: { action, note },
    });
  },

  adminSettings() {
    return apiRequest<Setting[]>("/admin/settings");
  },

  updateAdminSetting(key: string, value: string) {
    return apiRequest<void>(`/admin/settings/${encodeURIComponent(key)}`, {
      method: "PUT",
      body: { value },
    });
  },

  triggerSelectionSync() {
    return apiRequest<void>("/admin/selection/sync", { method: "POST" });
  },

  reindexReviews() {
    return apiRequest<void>("/admin/reviews/reindex", { method: "POST" });
  },

  reindexForum() {
    return apiRequest<void>("/admin/forum/reindex", { method: "POST" });
  },
};

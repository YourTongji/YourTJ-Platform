import type { components } from "./schema";

export type Schema = components["schemas"];

export type Account = Schema["Account"] & { trustLevel?: number };
export type Announcement = Schema["Announcement"];
export type Board = Schema["Board"];
export type Comment = Schema["Comment"];
export type Course = Schema["Course"];
export type CourseDetail = Schema["CourseDetail"];
export type Department = Schema["Department"];
export type LedgerEntry = Schema["LedgerEntry"];
export type LedgerVerify = Schema["LedgerVerify"];
export type Notification = Schema["Notification"];
export type Product = Schema["Product"];
export type Purchase = Schema["Purchase"];
export type Review = Schema["Review"];
export type SearchResult = Schema["SearchResult"];
export type Setting = Schema["Setting"];
export type Tag = Schema["Tag"];
export type Task = Schema["Task"];
export type Thread = Schema["Thread"];
export type ThreadDetail = Schema["ThreadDetail"];
export type ThreadFeed = Schema["ThreadFeed"];
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

export interface PollOption {
  id?: string;
  label?: string;
  body?: string;
  voteCount?: number;
  position?: number;
}

export interface Poll {
  id?: string;
  question?: string;
  multiSelect?: boolean;
  closesAt?: number | null;
  options?: PollOption[];
  myVotes?: string[];
}

export type ThreadDetailWithPoll = ThreadDetail & {
  poll?: Poll | null;
  featuredAt?: number | null;
  solvedAnswerId?: string | null;
};

export interface DmConversation {
  id?: string;
  participantHandle?: string;
  participantId?: string;
  lastMessageAt?: number;
  otherAccountId?: string;
  otherHandle?: string;
  otherAvatarUrl?: string | null;
  lastMessageBody?: string | null;
  unreadCount?: number;
  createdAt?: number;
}

export interface DmMessage {
  id?: string;
  conversationId?: string;
  senderId?: string;
  senderHandle?: string;
  body?: string;
  createdAt?: number;
}

export interface Bookmark {
  targetType?: string;
  targetId?: string;
  note?: string | null;
  createdAt?: number;
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

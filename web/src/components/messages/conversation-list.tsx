import {
  Archive,
  BellOff,
  Inbox,
  Loader2,
  MailQuestion,
  RotateCcw,
  Search,
  SendHorizontal,
  Trash2,
} from "lucide-react";
import type { ReactNode } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import type { DmConversation } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { cn } from "@/lib/utils";

export type ConversationView = "inbox" | "requests" | "sent" | "archived" | "deleted";

const viewLabels: Record<ConversationView, string> = {
  inbox: "收件箱",
  requests: "请求",
  sent: "已发送",
  archived: "归档",
  deleted: "删除",
};

export function ConversationList({
  conversations,
  selectedId,
  view,
  searchQuery,
  requestCount,
  headerAction,
  isLoading,
  error,
  hasMore,
  isLoadingMore,
  isRecovering,
  onRetry,
  onLoadMore,
  onSelect,
  onViewChange,
  onSearchChange,
  onRecover,
}: {
  conversations: DmConversation[];
  selectedId: string;
  view: ConversationView;
  searchQuery: string;
  requestCount: number;
  headerAction: ReactNode;
  isLoading: boolean;
  error?: unknown;
  hasMore: boolean;
  isLoadingMore: boolean;
  isRecovering: boolean;
  onRetry: () => void;
  onLoadMore: () => void;
  onSelect: (conversation: DmConversation) => void;
  onViewChange: (view: ConversationView) => void;
  onSearchChange: (query: string) => void;
  onRecover: (conversation: DmConversation) => void;
}) {
  return (
    <Card className="flex min-h-[32rem] flex-col overflow-hidden lg:h-[calc(100vh-10rem)]">
      <CardHeader className="gap-3 border-b">
        <div className="flex items-center justify-between gap-3">
          <CardTitle className="flex items-center gap-2">
            <Inbox className="size-4 text-primary" aria-hidden="true" />{viewLabels[view]}
          </CardTitle>
          {headerAction}
        </div>
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
          <Input
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
            className="h-9 pl-9"
            aria-label="搜索私信会话"
            placeholder="搜索联系人或最近消息"
            maxLength={100}
          />
        </div>
        <div className="grid grid-cols-5 gap-1 rounded-lg bg-muted p-1" aria-label="私信会话分类">
          {(["inbox", "requests", "sent", "archived", "deleted"] as const).map((item) => (
            <Button
              key={item}
              type="button"
              variant={view === item ? "secondary" : "ghost"}
              size="sm"
              className={cn("h-8", view === item && "bg-background shadow-sm")}
              aria-label={viewLabels[item]}
              aria-pressed={view === item}
              onClick={() => onViewChange(item)}
            >
              {item === "inbox" ? <Inbox className="size-3.5" /> : null}
              {item === "requests" ? <MailQuestion className="size-3.5" /> : null}
              {item === "sent" ? <SendHorizontal className="size-3.5" /> : null}
              {item === "archived" ? <Archive className="size-3.5" /> : null}
              {item === "deleted" ? <Trash2 className="size-3.5" /> : null}
              <span className="hidden min-[360px]:inline">{viewLabels[item]}</span>
              {item === "requests" && requestCount > 0 ? (
                <Badge variant="secondary" className="h-4 min-w-4 px-1 text-[10px] tabular-nums">
                  {requestCount > 99 ? "99+" : requestCount}
                </Badge>
              ) : null}
            </Button>
          ))}
        </div>
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-y-auto p-2">
        {isLoading ? (
          <LoadingState label={`加载${viewLabels[view]}`} />
        ) : error ? (
          <ErrorState error={error} onRetry={onRetry} />
        ) : conversations.length === 0 ? (
          <EmptyState
            title={searchQuery.trim() ? "没有匹配的会话" : `${viewLabels[view]}为空`}
            description={
              view === "inbox"
                ? "用公开 handle 发起一段一对一对话。"
                : view === "requests"
                  ? "陌生人只能先发送一条附言；接受后才能继续对话。"
                  : view === "sent"
                    ? "尚未被对方接受的请求会显示在这里。"
                    : "会话状态只影响你的收件箱，不会删除对方副本。"
            }
            className="border-0 shadow-none"
          />
        ) : (
          <div role="list" aria-label="私信会话" className="space-y-1">
            {conversations.map((conversation) => {
              const isSelected = selectedId === conversation.id;
              const content = (
                <>
                  <Avatar className="size-10 shrink-0">
                    <AvatarImage
                      src={conversation.participantAvatarUrl ?? undefined}
                      alt={`${conversation.participantHandle} 的头像`}
                    />
                    <AvatarFallback>{conversation.participantHandle.slice(0, 1).toUpperCase()}</AvatarFallback>
                  </Avatar>
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center justify-between gap-2">
                      <span className="flex min-w-0 items-center gap-1.5">
                        <span className="truncate text-sm font-medium">{conversation.participantHandle}</span>
                        {conversation.isMuted ? <BellOff className="size-3 shrink-0 text-muted-foreground" aria-label="已静音" /> : null}
                      </span>
                      <span className="shrink-0 text-[11px] text-muted-foreground">
                        {formatUnixTime(conversation.lastMessageAt ?? conversation.createdAt)}
                      </span>
                    </span>
                    <span className="mt-1 flex items-center gap-2">
                      <span className={cn(
                        "min-w-0 flex-1 truncate text-xs text-muted-foreground",
                        conversation.unreadCount > 0 && "font-medium text-foreground",
                      )}>
                        {conversation.lastMessageExcerpt || "还没有消息"}
                      </span>
                      {conversation.requestStatus === "pending" ? (
                        <Badge variant="outline" className="shrink-0 px-1.5 text-[10px]">
                          {conversation.requestDirection === "incoming" ? "待处理" : "待接受"}
                        </Badge>
                      ) : null}
                      {conversation.unreadCount > 0 ? (
                        <Badge className="min-w-5 justify-center px-1.5 tabular-nums">
                          {conversation.unreadCount > 99 ? "99+" : conversation.unreadCount}
                        </Badge>
                      ) : null}
                    </span>
                  </span>
                </>
              );
              return (
                <div key={conversation.id} role="listitem" className="flex items-center gap-1">
                  {view === "deleted" ? (
                    <div className="flex min-w-0 flex-1 items-start gap-3 rounded-lg p-3 text-left">{content}</div>
                  ) : (
                    <button
                      type="button"
                      aria-current={isSelected ? "true" : undefined}
                      onClick={() => onSelect(conversation)}
                      className={cn(
                        "flex min-w-0 flex-1 items-start gap-3 rounded-lg border border-transparent p-3 text-left outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50",
                        isSelected && "border-primary/20 bg-primary/10",
                      )}
                    >
                      {content}
                    </button>
                  )}
                  {view === "deleted" ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="shrink-0"
                      onClick={() => onRecover(conversation)}
                      disabled={isRecovering}
                      aria-label={`恢复与 ${conversation.participantHandle} 的会话`}
                    >
                      <RotateCcw className="size-4" />
                    </Button>
                  ) : null}
                </div>
              );
            })}
            {hasMore ? (
              <Button
                type="button"
                variant="ghost"
                className="w-full"
                onClick={onLoadMore}
                disabled={isLoadingMore}
              >
                {isLoadingMore ? <Loader2 className="size-4 animate-spin" /> : null}
                {isLoadingMore ? "加载中" : "加载更多会话"}
              </Button>
            ) : null}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

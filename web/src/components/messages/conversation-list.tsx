import { Inbox, Loader2 } from "lucide-react";
import type { ReactNode } from "react";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { DmConversation } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { cn } from "@/lib/utils";

export function ConversationList({
  conversations,
  selectedId,
  headerAction,
  isLoading,
  error,
  hasMore,
  isLoadingMore,
  onRetry,
  onLoadMore,
  onSelect,
}: {
  conversations: DmConversation[];
  selectedId: string;
  headerAction: ReactNode;
  isLoading: boolean;
  error?: unknown;
  hasMore: boolean;
  isLoadingMore: boolean;
  onRetry: () => void;
  onLoadMore: () => void;
  onSelect: (conversation: DmConversation) => void;
}) {
  return (
    <Card className="flex min-h-[32rem] flex-col overflow-hidden lg:h-[calc(100vh-10rem)]">
      <CardHeader className="flex-row items-center justify-between gap-3 border-b">
        <CardTitle className="flex items-center gap-2">
          <Inbox className="size-4 text-primary" aria-hidden="true" />收件箱
        </CardTitle>
        {headerAction}
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-y-auto p-2">
        {isLoading ? (
          <LoadingState label="加载私信会话" />
        ) : error ? (
          <ErrorState error={error} onRetry={onRetry} />
        ) : conversations.length === 0 ? (
          <EmptyState
            title="还没有私信"
            description="用公开 handle 发起一段一对一对话。"
            className="border-0 shadow-none"
          />
        ) : (
          <div role="list" aria-label="私信会话" className="space-y-1">
            {conversations.map((conversation) => {
              const isSelected = selectedId === conversation.id;
              return (
                <div key={conversation.id} role="listitem">
                  <button
                    type="button"
                    aria-current={isSelected ? "true" : undefined}
                    onClick={() => onSelect(conversation)}
                    className={cn(
                      "flex w-full items-start gap-3 rounded-lg border border-transparent p-3 text-left outline-none transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50",
                      isSelected && "border-primary/20 bg-primary/10",
                    )}
                  >
                    <Avatar className="size-10 shrink-0">
                      <AvatarImage
                        src={conversation.participantAvatarUrl ?? undefined}
                        alt={`${conversation.participantHandle} 的头像`}
                      />
                      <AvatarFallback>{conversation.participantHandle.slice(0, 1).toUpperCase()}</AvatarFallback>
                    </Avatar>
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center justify-between gap-2">
                        <span className="truncate text-sm font-medium">{conversation.participantHandle}</span>
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
                        {conversation.unreadCount > 0 ? (
                          <Badge className="min-w-5 justify-center px-1.5 tabular-nums">
                            {conversation.unreadCount > 99 ? "99+" : conversation.unreadCount}
                          </Badge>
                        ) : null}
                      </span>
                    </span>
                  </button>
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

import {
  Archive,
  ArchiveRestore,
  ArrowLeft,
  Ban,
  Bell,
  BellOff,
  Flag,
  Loader2,
  MoreHorizontal,
  Send,
  Trash2,
  UserRoundCheck,
} from "lucide-react";
import * as React from "react";
import { Link } from "react-router";

import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter, CardHeader } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Textarea } from "@/components/ui/textarea";
import type { DmConversation, DmMessage } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { cn } from "@/lib/utils";

export function ConversationThread({
  conversation,
  messages,
  currentAccountId,
  body,
  isIgnored,
  relationshipPending,
  lifecyclePending,
  isLoading,
  error,
  sendError,
  isSending,
  hasOlder,
  isLoadingOlder,
  onBodyChange,
  onBack,
  onRetry,
  onLoadOlder,
  onSend,
  onReport,
  onToggleIgnore,
  onToggleArchive,
  onToggleMute,
  onDelete,
}: {
  conversation?: DmConversation;
  messages: DmMessage[];
  currentAccountId?: string;
  body: string;
  isIgnored: boolean;
  relationshipPending: boolean;
  lifecyclePending: boolean;
  isLoading: boolean;
  error?: unknown;
  sendError?: unknown;
  isSending: boolean;
  hasOlder: boolean;
  isLoadingOlder: boolean;
  onBodyChange: (body: string) => void;
  onBack: () => void;
  onRetry: () => void;
  onLoadOlder: () => void;
  onSend: () => void;
  onReport: (message: DmMessage) => void;
  onToggleIgnore: () => void;
  onToggleArchive: () => void;
  onToggleMute: () => void;
  onDelete: () => void;
}) {
  const [confirmBlockOpen, setConfirmBlockOpen] = React.useState(false);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = React.useState(false);
  const viewportRef = React.useRef<HTMLDivElement>(null);
  const newestMessageId = messages.at(-1)?.id;

  React.useEffect(() => {
    if (newestMessageId) {
      viewportRef.current?.scrollTo({ top: viewportRef.current.scrollHeight });
    }
  }, [newestMessageId]);

  if (!conversation) {
    return (
      <Card className="hidden min-h-[32rem] items-center justify-center lg:flex lg:h-[calc(100vh-10rem)]">
        <EmptyState
          title="选择一个会话"
          description="从左侧收件箱继续对话，或用 handle 发起新私信。"
          className="border-0 shadow-none"
        />
      </Card>
    );
  }

  const canSend = Boolean(body.trim()) && !isSending && !isIgnored;

  return (
    <>
      <Card className="flex min-h-[32rem] flex-col overflow-hidden lg:h-[calc(100vh-10rem)]">
        <CardHeader className="flex-row items-center justify-between gap-3 border-b px-3 py-3 sm:px-5">
          <div className="flex min-w-0 items-center gap-2.5">
            <Button type="button" variant="ghost" size="icon" className="lg:hidden" onClick={onBack} aria-label="返回会话列表">
              <ArrowLeft className="size-4" />
            </Button>
            <Avatar className="size-9 shrink-0">
              <AvatarImage src={conversation.participantAvatarUrl ?? undefined} alt={`${conversation.participantHandle} 的头像`} />
              <AvatarFallback>{conversation.participantHandle.slice(0, 1).toUpperCase()}</AvatarFallback>
            </Avatar>
            <div className="min-w-0">
              <Link
                to={`/profile/${encodeURIComponent(conversation.participantHandle)}`}
                className="block truncate text-sm font-semibold outline-none hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50"
              >
                {conversation.participantHandle}
              </Link>
              <p className="text-xs text-muted-foreground">一对一私信</p>
            </div>
          </div>
          <div className="flex items-center gap-1">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button type="button" variant="ghost" size="icon" disabled={lifecyclePending} aria-label="会话设置">
                  <MoreHorizontal className="size-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={onToggleMute}>
                  {conversation.isMuted ? <Bell className="size-4" /> : <BellOff className="size-4" />}
                  {conversation.isMuted ? "恢复通知" : "静音通知"}
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={onToggleArchive}>
                  {conversation.isArchived ? <ArchiveRestore className="size-4" /> : <Archive className="size-4" />}
                  {conversation.isArchived ? "移回收件箱" : "归档会话"}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" onSelect={() => setConfirmDeleteOpen(true)}>
                  <Trash2 className="size-4" />删除会话
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button
              type="button"
              size="sm"
              variant={isIgnored ? "outline" : "ghost"}
              onClick={() => isIgnored ? onToggleIgnore() : setConfirmBlockOpen(true)}
              disabled={relationshipPending}
              aria-label={isIgnored ? `解除对 ${conversation.participantHandle} 的屏蔽` : `屏蔽 ${conversation.participantHandle}`}
            >
              {isIgnored ? <UserRoundCheck className="size-4" /> : <Ban className="size-4" />}
              <span className="hidden sm:inline">{isIgnored ? "解除屏蔽" : "屏蔽"}</span>
            </Button>
          </div>
        </CardHeader>

        <CardContent ref={viewportRef} className="min-h-0 flex-1 overflow-y-auto bg-muted/15 p-3 sm:p-5">
          {hasOlder ? (
            <div className="mb-4 flex justify-center">
              <Button type="button" variant="outline" size="sm" onClick={onLoadOlder} disabled={isLoadingOlder}>
                {isLoadingOlder ? <Loader2 className="size-4 animate-spin" /> : null}
                {isLoadingOlder ? "加载中" : "加载更早消息"}
              </Button>
            </div>
          ) : null}

          {isLoading ? (
            <LoadingState label="加载消息" />
          ) : error ? (
            <ErrorState error={error} onRetry={onRetry} />
          ) : messages.length === 0 ? (
            <EmptyState
              title="从一句问候开始"
              description="私信仅对会话双方可见；举报时只提交被举报的单条消息。"
              className="border-0 bg-transparent shadow-none"
            />
          ) : (
            <ol
              className="space-y-3"
              aria-label={`与 ${conversation.participantHandle} 的消息`}
              aria-live="polite"
              aria-relevant="additions"
            >
              {messages.map((message) => {
                const isMine = message.senderId === currentAccountId;
                return (
                  <li key={message.id} className={cn("group flex", isMine ? "justify-end" : "justify-start")}>
                    <div className={cn("max-w-[85%] sm:max-w-[72%]", isMine && "text-right")}>
                      <div className="mb-1 flex items-center gap-2 text-[11px] text-muted-foreground">
                        {!isMine ? <span className="font-medium">{message.senderHandle}</span> : null}
                        <time dateTime={new Date(message.createdAt * 1000).toISOString()}>
                          {formatUnixTime(message.createdAt)}
                        </time>
                        {!isMine ? (
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon"
                            className="size-6 opacity-70 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100"
                            onClick={() => onReport(message)}
                            aria-label={`举报 ${message.senderHandle} 的这条消息`}
                          >
                            <Flag className="size-3" />
                          </Button>
                        ) : null}
                      </div>
                      <div className={cn(
                        "whitespace-pre-wrap break-words rounded-2xl px-3.5 py-2.5 text-left text-sm leading-6 shadow-sm",
                        isMine
                          ? "rounded-br-sm bg-primary text-primary-foreground"
                          : "rounded-bl-sm border bg-card text-card-foreground",
                      )}>
                        {message.body}
                      </div>
                    </div>
                  </li>
                );
              })}
            </ol>
          )}
        </CardContent>

        <CardFooter className="block border-t p-3 sm:p-4">
          {isIgnored ? (
            <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
              <span>你已屏蔽该用户，解除屏蔽后才能继续对话。</span>
              <Button type="button" variant="outline" size="sm" onClick={onToggleIgnore} disabled={relationshipPending}>
                解除屏蔽
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              <label htmlFor="dm-message-body" className="sr-only">消息内容</label>
              <div className="flex items-end gap-2">
                <Textarea
                  id="dm-message-body"
                  value={body}
                  onChange={(event) => onBodyChange(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                      event.preventDefault();
                      if (canSend) onSend();
                    }
                  }}
                  placeholder="输入消息；Enter 发送，Shift + Enter 换行"
                  maxLength={16000}
                  rows={2}
                  aria-describedby={sendError ? "dm-send-error" : "dm-send-help"}
                />
                <Button type="button" size="icon" onClick={onSend} disabled={!canSend} aria-label="发送消息">
                  {isSending ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}
                </Button>
              </div>
              <div className="flex items-center justify-between gap-3 text-xs">
                {sendError ? (
                  <p id="dm-send-error" role="alert" className="text-destructive">
                    {sendError instanceof Error ? sendError.message : "消息发送失败"}
                  </p>
                ) : (
                  <p id="dm-send-help" className="text-muted-foreground">请勿发送校园身份、联系方式等不必要的敏感信息。</p>
                )}
                <span className="shrink-0 tabular-nums text-muted-foreground">{body.length}/16000</span>
              </div>
            </div>
          )}
        </CardFooter>
      </Card>

      <Dialog open={confirmBlockOpen} onOpenChange={setConfirmBlockOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>屏蔽 {conversation.participantHandle}？</DialogTitle>
            <DialogDescription>
              双方将无法继续发送私信，该用户的帖子和回复也会从你的社区信息流中隐藏。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setConfirmBlockOpen(false)}>取消</Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => {
                onToggleIgnore();
                setConfirmBlockOpen(false);
              }}
              disabled={relationshipPending}
            >
              <Ban className="size-4" />确认屏蔽
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={confirmDeleteOpen} onOpenChange={setConfirmDeleteOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除这段会话？</DialogTitle>
            <DialogDescription>
              会话只会从你的收件箱隐藏，对方副本不会被删除。你可以在“最近删除”中恢复；对方发送新消息时会话也会重新出现。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setConfirmDeleteOpen(false)}>取消</Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => {
                onDelete();
                setConfirmDeleteOpen(false);
              }}
              disabled={lifecyclePending}
            >
              <Trash2 className="size-4" />删除我的副本
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

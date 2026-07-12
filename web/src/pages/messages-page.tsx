import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { Link, useSearchParams } from "react-router";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, LoadingState } from "@/components/common/states";
import { ConversationList, type ConversationView } from "@/components/messages/conversation-list";
import { ConversationThread } from "@/components/messages/conversation-thread";
import { NewConversationDialog } from "@/components/messages/new-conversation-dialog";
import { ReportMessageDialog } from "@/components/messages/report-message-dialog";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/context/auth-provider";
import { accountQueryKeys } from "@/lib/account-query-keys";
import { api } from "@/lib/api/endpoints";
import type { DmConversation, DmMessage, DmReportReason } from "@/lib/api/types";
import { cn } from "@/lib/utils";

export function MessagesPage() {
  const { account, isAuthenticated, isLoading: authLoading } = useAuth();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedId = searchParams.get("conversation") ?? "";
  const rawView = searchParams.get("view");
  const view: ConversationView = rawView === "requests"
    || rawView === "sent"
    || rawView === "archived"
    || rawView === "deleted"
    ? rawView
    : "inbox";
  const composeRecipient = searchParams.get("recipient") ?? undefined;
  const [body, setBody] = React.useState("");
  const [conversationSearch, setConversationSearch] = React.useState("");
  const deferredConversationSearch = React.useDeferredValue(conversationSearch.trim());
  const [reportingMessage, setReportingMessage] = React.useState<DmMessage | null>(null);
  const [reportingRequest, setReportingRequest] = React.useState(false);
  const lastMarkedRead = React.useRef("");
  const directMessageKey = React.useMemo(
    () => accountQueryKeys.directMessages(account?.id),
    [account?.id],
  );
  const directMessageCountKey = React.useMemo(
    () => accountQueryKeys.directMessageCount(account?.id),
    [account?.id],
  );
  const ignoredUsersKey = React.useMemo(
    () => accountQueryKeys.ignoredUsers(account?.id),
    [account?.id],
  );

  const conversations = useInfiniteQuery({
    queryKey: [...directMessageKey, "conversations", view, deferredConversationSearch],
    queryFn: ({ pageParam }) => api.dmConversations({
      cursor: pageParam,
      view,
      q: deferredConversationSearch.length >= 2 ? deferredConversationSearch : undefined,
    }),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: isAuthenticated,
  });
  const conversationItems = conversations.data?.pages.flatMap((page) => page.items ?? []) ?? [];
  const selectedConversation = conversationItems.find((item) => item.id === selectedId);
  const fetchNextConversationPage = conversations.fetchNextPage;
  const hasNextConversationPage = conversations.hasNextPage;
  const isFetchingNextConversationPage = conversations.isFetchingNextPage;
  const dmCounts = useQuery({
    queryKey: directMessageCountKey,
    queryFn: api.dmUnreadCount,
    enabled: isAuthenticated,
    staleTime: 30_000,
  });

  React.useEffect(() => {
    if (selectedId && !selectedConversation && hasNextConversationPage && !isFetchingNextConversationPage) {
      void fetchNextConversationPage();
    }
  }, [
    fetchNextConversationPage,
    hasNextConversationPage,
    isFetchingNextConversationPage,
    selectedConversation,
    selectedId,
  ]);

  const messages = useInfiniteQuery({
    queryKey: [...directMessageKey, "messages", selectedId],
    queryFn: ({ pageParam }) => api.dmMessages(selectedId, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: isAuthenticated && Boolean(selectedId),
  });
  const messageItems = React.useMemo(
    () => (messages.data?.pages.flatMap((page) => page.items ?? []) ?? []).reverse(),
    [messages.data?.pages],
  );

  const ignoredUsers = useInfiniteQuery({
    queryKey: ignoredUsersKey,
    queryFn: ({ pageParam }) => api.ignoredUsers(pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: isAuthenticated,
  });
  const fetchNextIgnoredPage = ignoredUsers.fetchNextPage;
  const hasNextIgnoredPage = ignoredUsers.hasNextPage;
  const isFetchingNextIgnoredPage = ignoredUsers.isFetchingNextPage;
  React.useEffect(() => {
    if (hasNextIgnoredPage && !isFetchingNextIgnoredPage) {
      void fetchNextIgnoredPage();
    }
  }, [fetchNextIgnoredPage, hasNextIgnoredPage, isFetchingNextIgnoredPage]);
  const selectedIsIgnored = Boolean(
    selectedConversation
    && ignoredUsers.data?.pages.some((page) =>
      (page.items ?? []).some((item) => item.accountId === selectedConversation.participantId)),
  );

  const createConversation = useMutation({
    mutationFn: ({ handle, requestMessage, idempotencyKey }: {
      handle: string;
      requestMessage: string;
      idempotencyKey: string;
    }) => api.createDmConversation(handle, requestMessage, idempotencyKey),
    onSuccess: async (conversation) => {
      toast.success(conversation.requestStatus === "pending"
        ? "消息请求已发送；对方接受前不能继续发送"
        : `已打开与 ${conversation.participantHandle} 的对话`);
      selectConversation(conversation, conversation.requestStatus === "pending" ? "sent" : "inbox");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
        queryClient.invalidateQueries({ queryKey: directMessageCountKey }),
      ]);
    },
  });
  const sendMessage = useMutation({
    mutationFn: () => api.sendDmMessage(selectedId, body.trim()),
    onSuccess: async () => {
      setBody("");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "messages", selectedId] }),
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
        queryClient.invalidateQueries({ queryKey: directMessageCountKey }),
      ]);
    },
  });
  const reportMessage = useMutation({
    mutationFn: ({ message, reason, note }: {
      message: DmMessage;
      reason: DmReportReason;
      note?: string;
    }) => reportingRequest && selectedConversation
      ? api.reportDmRequest(selectedConversation.id, reason, note)
      : api.reportDmMessage(message.id, reason, note),
    onSuccess: async () => {
      toast.success("举报已提交，审核人员只会看到这条消息及你的说明");
      if (reportingRequest) clearSelection();
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
        queryClient.invalidateQueries({ queryKey: directMessageCountKey }),
      ]);
    },
  });
  const requestAction = useMutation({
    mutationFn: async (action: "accept" | "decline") => {
      if (!selectedConversation) throw new Error("消息请求不存在");
      if (action === "accept") {
        return { action, conversation: await api.acceptDmRequest(selectedConversation.id) };
      }
      await api.declineDmRequest(selectedConversation.id);
      return { action, conversation: undefined };
    },
    onSuccess: async ({ action, conversation }) => {
      if (action === "accept" && conversation) {
        toast.success("已接受消息请求，现在可以正常回复");
        selectConversation(conversation, "inbox");
      } else {
        toast.success(view === "sent" ? "消息请求已撤回" : "消息请求已删除");
        clearSelection();
      }
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
        queryClient.invalidateQueries({ queryKey: directMessageCountKey }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "消息请求操作失败"),
  });
  const relationship = useMutation({
    mutationFn: async () => {
      if (!selectedConversation) return;
      if (selectedIsIgnored) {
        await api.unignoreUser(selectedConversation.participantId);
      } else {
        await api.ignoreUser(selectedConversation.participantId);
      }
    },
    onSuccess: async () => {
      toast.success(selectedIsIgnored ? "已解除屏蔽" : "已屏蔽该用户");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ignoredUsersKey }),
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "关系设置失败"),
  });
  const lifecycle = useMutation({
    mutationFn: async ({
      action,
      conversation,
    }: {
      action: "archive" | "mute" | "delete" | "recover";
      conversation: DmConversation;
    }) => {
      if (action === "archive") {
        await api.setDmConversationArchived(conversation.id, !conversation.isArchived);
      } else if (action === "mute") {
        await api.setDmConversationMuted(conversation.id, !conversation.isMuted);
      } else if (action === "delete") {
        await api.deleteDmConversation(conversation.id);
      } else {
        await api.recoverDmConversation(conversation.id);
      }
      return { action, conversation };
    },
    onSuccess: async ({ action, conversation }) => {
      const messages: Record<typeof action, string> = {
        archive: conversation.isArchived ? "会话已移回收件箱" : "会话已归档",
        mute: conversation.isMuted ? "已恢复会话通知" : "已静音会话通知",
        delete: "会话已移到最近删除",
        recover: "会话已恢复",
      };
      toast.success(messages[action]);
      if (action === "delete" || action === "archive") clearSelection();
      if (action === "recover") selectConversation(conversation, "inbox");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: [...directMessageKey, "conversations"] }),
        queryClient.invalidateQueries({ queryKey: directMessageCountKey }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "会话操作失败"),
  });

  const newestMessage = messages.data?.pages[0]?.items?.[0];
  React.useEffect(() => {
    if (!selectedId || !newestMessage?.id || selectedConversation?.requestStatus !== "accepted") return;
    const readKey = `${selectedId}:${newestMessage.id}`;
    if (lastMarkedRead.current === readKey) return;
    lastMarkedRead.current = readKey;
    void api
      .markDmConversationRead(selectedId, newestMessage.id)
      .then(() => queryClient.invalidateQueries({
        queryKey: [...directMessageKey, "conversations"],
      }))
      .catch(() => {
        lastMarkedRead.current = "";
      });
  }, [directMessageKey, newestMessage?.id, queryClient, selectedConversation?.requestStatus, selectedId]);

  function selectConversation(conversation: DmConversation, nextView: ConversationView = view) {
    setBody("");
    sendMessage.reset();
    const next = new URLSearchParams({ conversation: conversation.id });
    if (nextView !== "inbox") next.set("view", nextView);
    setSearchParams(next, { replace: true });
  }

  function clearSelection() {
    setBody("");
    sendMessage.reset();
    const next = new URLSearchParams();
    if (view !== "inbox") next.set("view", view);
    setSearchParams(next, { replace: true });
  }

  function changeView(nextView: ConversationView) {
    setBody("");
    setConversationSearch("");
    const next = new URLSearchParams();
    if (nextView !== "inbox") next.set("view", nextView);
    setSearchParams(next, { replace: true });
  }

  function dismissComposer() {
    if (!composeRecipient) return;
    const next = new URLSearchParams(searchParams);
    next.delete("recipient");
    setSearchParams(next, { replace: true });
  }

  if (authLoading) {
    return <LoadingState label="确认登录状态" />;
  }
  if (!isAuthenticated) {
    return (
      <EmptyState
        title="登录后使用私信"
        description="私信只对会话双方可见；举报时只向审核人员提交被举报的单条消息。"
        action={<Button asChild><Link to="/login">前往登录</Link></Button>}
      />
    );
  }

  return (
    <div>
      <PageHeader
        title="私信"
        description="一对一站内沟通。陌生联系先进入消息请求，未举报的对话不会向管理员开放。"
      />
      <div className="grid gap-5 lg:grid-cols-[22rem_minmax(0,1fr)]">
        <aside className={cn(selectedConversation && "hidden lg:block")}>
          <ConversationList
            conversations={conversationItems}
            selectedId={selectedId}
            view={view}
            searchQuery={conversationSearch}
            requestCount={dmCounts.data?.requestCount ?? 0}
            headerAction={(
              view === "inbox" ? (
                <NewConversationDialog
                  canCreate={(account?.trustLevel ?? 0) >= 1}
                  initialHandle={composeRecipient}
                  isPending={createConversation.isPending}
                  error={createConversation.error}
                  onReset={createConversation.reset}
                  onDismiss={dismissComposer}
                  onCreate={(handle, requestMessage, idempotencyKey) => createConversation.mutateAsync({
                    handle,
                    requestMessage,
                    idempotencyKey,
                  })}
                />
              ) : null
            )}
            isLoading={conversations.isLoading}
            error={conversations.error}
            hasMore={Boolean(conversations.hasNextPage)}
            isLoadingMore={conversations.isFetchingNextPage}
            isRecovering={lifecycle.isPending}
            onRetry={() => void conversations.refetch()}
            onLoadMore={() => void conversations.fetchNextPage()}
            onSelect={selectConversation}
            onViewChange={changeView}
            onSearchChange={setConversationSearch}
            onRecover={(conversation) => lifecycle.mutate({ action: "recover", conversation })}
          />
        </aside>

        <section className={cn(!selectedConversation && "hidden lg:block")}>
          <ConversationThread
            conversation={selectedConversation}
            messages={messageItems}
            currentAccountId={account?.id}
            body={body}
            isIgnored={selectedIsIgnored}
            relationshipPending={relationship.isPending || ignoredUsers.isLoading || ignoredUsers.isFetchingNextPage}
            lifecyclePending={lifecycle.isPending}
            requestActionPending={requestAction.isPending || reportMessage.isPending}
            isLoading={messages.isLoading}
            error={messages.error}
            sendError={sendMessage.error}
            isSending={sendMessage.isPending}
            hasOlder={Boolean(messages.hasNextPage)}
            isLoadingOlder={messages.isFetchingNextPage}
            onBodyChange={setBody}
            onBack={clearSelection}
            onRetry={() => void messages.refetch()}
            onLoadOlder={() => void messages.fetchNextPage()}
            onSend={() => sendMessage.mutate()}
            onReport={(message, isRequest) => {
              reportMessage.reset();
              setReportingRequest(isRequest);
              setReportingMessage(message);
            }}
            onAcceptRequest={() => requestAction.mutate("accept")}
            onDeclineRequest={() => requestAction.mutate("decline")}
            onToggleIgnore={() => relationship.mutate()}
            onToggleArchive={() => {
              if (selectedConversation) lifecycle.mutate({ action: "archive", conversation: selectedConversation });
            }}
            onToggleMute={() => {
              if (selectedConversation) lifecycle.mutate({ action: "mute", conversation: selectedConversation });
            }}
            onDelete={() => {
              if (selectedConversation) lifecycle.mutate({ action: "delete", conversation: selectedConversation });
            }}
          />
        </section>
      </div>

      <ReportMessageDialog
        message={reportingMessage}
        isPending={reportMessage.isPending}
        error={reportMessage.error}
        onClose={() => {
          setReportingMessage(null);
          setReportingRequest(false);
          reportMessage.reset();
        }}
        onReport={(message, reason, note) => reportMessage.mutateAsync({ message, reason, note })}
      />
    </div>
  );
}

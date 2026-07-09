import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MessageSquare, Plus, Send } from "lucide-react";
import * as React from "react";
import { toast } from "sonner";

import { PageHeader } from "@/components/common/page-header";
import { EmptyState, ErrorState, LoadingState } from "@/components/common/states";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useAuth } from "@/context/auth-provider";
import { api } from "@/lib/api/endpoints";
import type { DmConversation } from "@/lib/api/types";
import { formatUnixTime } from "@/lib/format";
import { cn } from "@/lib/utils";

function conversationLabel(item: DmConversation) {
  return item.participantHandle ?? item.otherHandle ?? item.participantId ?? item.otherAccountId ?? item.id ?? "会话";
}

export function MessagesPage() {
  const { isAuthenticated } = useAuth();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = React.useState("");
  const [recipientId, setRecipientId] = React.useState("");
  const [body, setBody] = React.useState("");
  const conversations = useQuery({
    queryKey: ["dm", "conversations"],
    queryFn: api.dmConversations,
    enabled: isAuthenticated,
  });
  const messages = useQuery({
    queryKey: ["dm", "messages", selectedId],
    queryFn: () => api.dmMessages(selectedId),
    enabled: isAuthenticated && Boolean(selectedId),
  });
  const createConversation = useMutation({
    mutationFn: () => api.createDmConversation(recipientId),
    onSuccess: async (data) => {
      toast.success("会话已创建");
      setRecipientId("");
      await queryClient.invalidateQueries({ queryKey: ["dm", "conversations"] });
      if (data.id) {
        setSelectedId(data.id);
      }
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "创建会话失败"),
  });
  const sendMessage = useMutation({
    mutationFn: () => api.sendDmMessage(selectedId, body),
    onSuccess: async () => {
      setBody("");
      await queryClient.invalidateQueries({ queryKey: ["dm", "messages", selectedId] });
      await queryClient.invalidateQueries({ queryKey: ["dm", "conversations"] });
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "发送失败"),
  });

  React.useEffect(() => {
    if (!selectedId && conversations.data?.[0]?.id) {
      setSelectedId(conversations.data[0].id);
    }
  }, [conversations.data, selectedId]);

  if (!isAuthenticated) {
    return <EmptyState title="登录后使用私信" />;
  }

  return (
    <div>
      <PageHeader
        eyebrow="DM"
        title="私信"
        description="1:1 站内私信，接入 forum DM conversation/message 接口。"
      />
      <div className="grid gap-5 lg:grid-cols-[20rem_minmax(0,1fr)]">
        <aside className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Plus className="h-4 w-4 text-primary" />
                新建会话
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <Label>对方账号 ID</Label>
              <div className="flex gap-2">
                <Input value={recipientId} onChange={(event) => setRecipientId(event.target.value)} />
                <Button onClick={() => createConversation.mutate()} disabled={!recipientId || createConversation.isPending}>
                  创建
                </Button>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>会话</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {conversations.isLoading ? (
                <LoadingState />
              ) : conversations.isError ? (
                <ErrorState error={conversations.error} onRetry={() => void conversations.refetch()} />
              ) : (conversations.data ?? []).length === 0 ? (
                <p className="text-sm text-muted-foreground">暂无会话</p>
              ) : (
                conversations.data?.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => item.id && setSelectedId(item.id)}
                    className={cn(
                      "block w-full rounded-md border p-3 text-left text-sm transition-colors hover:bg-accent",
                      selectedId === item.id && "border-primary bg-secondary",
                    )}
                  >
                    <p className="font-medium">{conversationLabel(item)}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {formatUnixTime(item.lastMessageAt ?? item.createdAt)}
                    </p>
                  </button>
                ))
              )}
            </CardContent>
          </Card>
        </aside>

        <section className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <MessageSquare className="h-4 w-4 text-primary" />
                消息
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {!selectedId ? (
                <EmptyState title="选择一个会话" />
              ) : messages.isLoading ? (
                <LoadingState />
              ) : messages.isError ? (
                <ErrorState error={messages.error} onRetry={() => void messages.refetch()} />
              ) : (messages.data?.items ?? []).length === 0 ? (
                <EmptyState title="暂无消息" />
              ) : (
                messages.data?.items?.map((message) => (
                  <div key={message.id} className="rounded-md border p-3">
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-medium">{message.senderHandle}</p>
                      <p className="text-xs text-muted-foreground">{formatUnixTime(message.createdAt)}</p>
                    </div>
                    <p className="mt-2 whitespace-pre-wrap text-sm">{message.body}</p>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
          {selectedId ? (
            <Card>
              <CardContent className="space-y-3 p-4">
                <Textarea value={body} onChange={(event) => setBody(event.target.value)} placeholder="输入消息" />
                <Button onClick={() => sendMessage.mutate()} disabled={!body.trim() || sendMessage.isPending}>
                  <Send className="h-4 w-4" />
                  发送
                </Button>
              </CardContent>
            </Card>
          ) : null}
        </section>
      </div>
    </div>
  );
}

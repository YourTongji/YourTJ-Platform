import { MessageSquarePlus } from "lucide-react";
import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { DmConversation } from "@/lib/api/types";
import { idempotencyKey } from "@/lib/format";

export function NewConversationDialog({
  canCreate,
  initialHandle,
  isPending,
  error,
  onReset,
  onDismiss,
  onCreate,
}: {
  canCreate: boolean;
  initialHandle?: string;
  isPending: boolean;
  error?: unknown;
  onReset: () => void;
  onDismiss?: () => void;
  onCreate: (handle: string, requestMessage: string, idempotencyKey: string) => Promise<DmConversation>;
}) {
  const [open, setOpen] = React.useState(Boolean(initialHandle));
  const [handle, setHandle] = React.useState(initialHandle ?? "");
  const [requestMessage, setRequestMessage] = React.useState("");
  const normalizedHandle = handle.trim();
  const normalizedMessage = requestMessage.trim();
  const handleLength = [...normalizedHandle].length;
  const messageLength = [...normalizedMessage].length;
  const isValid = handleLength >= 3 && handleLength <= 30
    && messageLength >= 1 && messageLength <= 1000;

  React.useEffect(() => {
    if (!initialHandle) return;
    setHandle(initialHandle);
    setOpen(true);
    onReset();
  }, [initialHandle, onReset]);

  function closeDialog() {
    setOpen(false);
    onDismiss?.();
  }

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!isValid) return;
    try {
      await onCreate(normalizedHandle, normalizedMessage, idempotencyKey("dm-request"));
      setHandle("");
      setRequestMessage("");
      setOpen(false);
    } catch {
      // The mutation error remains visible in the dialog so the user can correct the handle.
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (nextOpen) onReset();
        else onDismiss?.();
      }}
    >
      <DialogTrigger asChild>
        <Button type="button" size="sm" disabled={!canCreate} title={canCreate ? undefined : "达到信任等级 1 后开放"}>
          <MessageSquarePlus className="size-4" />新建私信
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form onSubmit={(event) => void submit(event)}>
          <DialogHeader>
            <DialogTitle>发起私信</DialogTitle>
            <DialogDescription>
              若对方尚未关注你，这段附言会进入独立的消息请求；对方接受前你不能继续发送。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2 py-5">
            <Label htmlFor="dm-recipient-handle">对方 handle</Label>
            <Input
              id="dm-recipient-handle"
              value={handle}
              onChange={(event) => setHandle(event.target.value)}
              placeholder="例如 tongji_friend"
              autoComplete="off"
              minLength={3}
              maxLength={30}
              aria-describedby={error ? "dm-recipient-error" : undefined}
            />
            <Label htmlFor="dm-request-message">请求附言</Label>
            <Textarea
              id="dm-request-message"
              value={requestMessage}
              onChange={(event) => setRequestMessage(event.target.value)}
              placeholder="说明来意，避免发送校园身份或联系方式等敏感信息"
              maxLength={1000}
              rows={4}
              required
              aria-describedby={error ? "dm-recipient-error" : "dm-request-message-help"}
            />
            {error ? (
              <p id="dm-recipient-error" role="alert" className="text-sm text-destructive">
                {error instanceof Error ? error.message : "无法创建会话"}
              </p>
            ) : (
              <p id="dm-request-message-help" className="flex justify-between gap-3 text-xs text-muted-foreground">
                <span>无需填写账号 ID，也不会展示校园邮箱。</span>
                <span className="shrink-0 tabular-nums">{messageLength}/1000</span>
              </p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={closeDialog}>取消</Button>
            <Button type="submit" disabled={!isValid || isPending}>
              {isPending ? "发送中" : "发送并开始"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

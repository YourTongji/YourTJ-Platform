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
import type { DmConversation } from "@/lib/api/types";

export function NewConversationDialog({
  canCreate,
  isPending,
  error,
  onReset,
  onCreate,
}: {
  canCreate: boolean;
  isPending: boolean;
  error?: unknown;
  onReset: () => void;
  onCreate: (handle: string) => Promise<DmConversation>;
}) {
  const [open, setOpen] = React.useState(false);
  const [handle, setHandle] = React.useState("");
  const normalizedHandle = handle.trim();
  const isValid = normalizedHandle.length >= 3 && normalizedHandle.length <= 30;

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!isValid) return;
    try {
      await onCreate(normalizedHandle);
      setHandle("");
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
              输入对方的公开 handle。已屏蔽关系、被封禁账号或无效 handle 会由服务端拒绝。
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
            {error ? (
              <p id="dm-recipient-error" role="alert" className="text-sm text-destructive">
                {error instanceof Error ? error.message : "无法创建会话"}
              </p>
            ) : (
              <p className="text-xs text-muted-foreground">无需填写账号 ID，也不会展示校园邮箱。</p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setOpen(false)}>取消</Button>
            <Button type="submit" disabled={!isValid || isPending}>
              {isPending ? "创建中" : "开始对话"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Archive,
  Eye,
  EyeOff,
  Lock,
  LockOpen,
  MoreHorizontal,
  MoveRight,
  Pin,
  PinOff,
  RotateCcw,
  ShieldCheck,
  Trash2,
  UserCog,
  UserRound,
} from "lucide-react";
import * as React from "react";
import { Link } from "react-router";
import { toast } from "sonner";

import { ReasonDialog } from "@/components/admin/admin-primitives";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api/endpoints";
import type { Board, Comment, ThreadDetailWithPoll } from "@/lib/api/types";
import { forumQueryKeys } from "@/lib/forum-query-keys";

type ThreadModerationAction =
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
  | "move";

type CommentModerationAction = "delete" | "restore" | "hide" | "unhide";

interface ActionCopy {
  title: string;
  description: string;
  confirmLabel: string;
  destructive?: boolean;
}

const threadActionCopy: Record<ThreadModerationAction, ActionCopy> = {
  pin: {
    title: "置顶帖子",
    description: "帖子将出现在当前板块顶部；启用全站置顶后会扩大到所有板块。",
    confirmLabel: "确认置顶",
  },
  unpin: {
    title: "取消置顶",
    description: "帖子将恢复到常规排序，已有回复和订阅不会受到影响。",
    confirmLabel: "取消置顶",
  },
  close: {
    title: "关闭帖子",
    description: "关闭后不能继续回复，但现有内容和审计记录仍会保留。",
    confirmLabel: "确认关闭",
    destructive: true,
  },
  reopen: {
    title: "重新开放帖子",
    description: "重新开放后，符合发言条件的用户可以继续回复。",
    confirmLabel: "确认开放",
  },
  archive: {
    title: "归档帖子",
    description: "归档会把帖子移出常规信息流并停止新回复，请确认该讨论不再需要保持活跃。",
    confirmLabel: "确认归档",
    destructive: true,
  },
  unarchive: {
    title: "取消归档",
    description: "帖子会重新进入常规信息流；关闭、隐藏或删除状态不会被一并改变。",
    confirmLabel: "取消归档",
  },
  delete: {
    title: "删除帖子",
    description: "帖子会被软删除并移出公开页面，内容与治理审计仍会保留。",
    confirmLabel: "确认删除",
    destructive: true,
  },
  restore: {
    title: "恢复帖子",
    description: "恢复被软删除的帖子，并重新计算它对作者活跃度的贡献。",
    confirmLabel: "确认恢复",
  },
  hide: {
    title: "隐藏帖子",
    description: "帖子会从公共页面隐藏，并从作者活跃度贡献中移除。",
    confirmLabel: "确认隐藏",
    destructive: true,
  },
  unhide: {
    title: "取消隐藏",
    description: "帖子会重新公开，并按原发布时间恢复对应活跃度贡献。",
    confirmLabel: "恢复公开",
  },
  move: {
    title: "移动帖子",
    description: "选择目标板块。移动不会修改帖子作者、正文或历史记录。",
    confirmLabel: "确认移动",
  },
};

const commentActionCopy: Record<CommentModerationAction, ActionCopy> = {
  delete: {
    title: "删除回复",
    description: "回复会被软删除，楼层记录与治理审计仍会保留。",
    confirmLabel: "确认删除",
    destructive: true,
  },
  restore: {
    title: "恢复回复",
    description: "恢复被软删除的回复，并在内容可见时恢复活跃度贡献。",
    confirmLabel: "确认恢复",
  },
  hide: {
    title: "隐藏回复",
    description: "回复会从公共页面隐藏，并从作者活跃度贡献中移除。",
    confirmLabel: "确认隐藏",
    destructive: true,
  },
  unhide: {
    title: "取消隐藏",
    description: "回复会重新公开，并在未删除时恢复对应活跃度贡献。",
    confirmLabel: "恢复公开",
  },
};

function StaffTargetLinks({ authorHandle }: { authorHandle?: string }) {
  if (!authorHandle) {
    return null;
  }

  const encodedHandle = encodeURIComponent(authorHandle);
  return (
    <>
      <DropdownMenuSeparator />
      <DropdownMenuLabel>用户处置</DropdownMenuLabel>
      <DropdownMenuItem asChild>
        <Link to={`/profile/${encodedHandle}`}>
          <UserRound className="size-4" aria-hidden="true" />
          查看公开资料
        </Link>
      </DropdownMenuItem>
      <DropdownMenuItem asChild>
        <Link to={`/admin?section=users&q=${encodedHandle}`}>
          <UserCog className="size-4" aria-hidden="true" />
          前往用户管理
        </Link>
      </DropdownMenuItem>
    </>
  );
}

function ThreadActionIcon({ action }: { action: ThreadModerationAction }) {
  switch (action) {
    case "pin":
      return <Pin className="size-4" aria-hidden="true" />;
    case "unpin":
      return <PinOff className="size-4" aria-hidden="true" />;
    case "close":
      return <Lock className="size-4" aria-hidden="true" />;
    case "reopen":
      return <LockOpen className="size-4" aria-hidden="true" />;
    case "archive":
      return <Archive className="size-4" aria-hidden="true" />;
    case "unarchive":
      return <RotateCcw className="size-4" aria-hidden="true" />;
    case "delete":
      return <Trash2 className="size-4" aria-hidden="true" />;
    case "restore":
      return <RotateCcw className="size-4" aria-hidden="true" />;
    case "hide":
      return <EyeOff className="size-4" aria-hidden="true" />;
    case "unhide":
      return <Eye className="size-4" aria-hidden="true" />;
    case "move":
      return <MoveRight className="size-4" aria-hidden="true" />;
  }
}

function threadActionLabel(action: ThreadModerationAction, isPinnedGlobally: boolean) {
  if (action === "unpin" && isPinnedGlobally) {
    return "取消全站置顶";
  }
  return threadActionCopy[action].title;
}

export function ThreadModerationMenu({
  thread,
  boards,
}: {
  thread: ThreadDetailWithPoll;
  boards: Board[];
}) {
  const queryClient = useQueryClient();
  const [selectedAction, setSelectedAction] = React.useState<ThreadModerationAction | null>(null);
  const [moveBoardId, setMoveBoardId] = React.useState("");
  const [isGlobalPin, setIsGlobalPin] = React.useState(false);
  const threadId = thread.id ?? "";
  const eligibleBoards = boards.filter(
    (board) => Boolean(board.id) && board.id !== thread.boardId,
  );

  const actions = React.useMemo<ThreadModerationAction[]>(() => {
    if (thread.deletedAt) {
      return ["restore"];
    }

    const next: ThreadModerationAction[] = [];
    if (!thread.archivedAt) {
      next.push(thread.pinnedAt ? "unpin" : "pin");
      next.push(thread.closedAt ? "reopen" : "close");
    } else {
      next.push("unarchive");
    }
    next.push(thread.hiddenAt ? "unhide" : "hide");
    if (!thread.archivedAt) {
      next.push("archive");
    }
    next.push("delete");
    if (eligibleBoards.length > 0) {
      next.push("move");
    }
    return next;
  }, [eligibleBoards.length, thread.archivedAt, thread.closedAt, thread.deletedAt, thread.hiddenAt, thread.pinnedAt]);

  const mutation = useMutation({
    mutationFn: ({
      action,
      reason,
      boardId,
      globally,
    }: {
      action: ThreadModerationAction;
      reason: string;
      boardId?: string;
      globally?: boolean;
    }) => api.moderateForumThread(threadId, action, { reason, boardId, globally }),
    onSuccess: async (_response, variables) => {
      toast.success(`${threadActionCopy[variables.action].title}已完成`);
      setSelectedAction(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.thread(threadId) }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.comments(threadId) }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.feeds() }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.boards() }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.homeFeeds() }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.profile(thread.authorHandle) }),
        queryClient.invalidateQueries({ queryKey: ["admin", "forum-flags"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "forum", "thread", threadId] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "overview"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "管理操作失败"),
  });

  function selectAction(action: ThreadModerationAction) {
    setSelectedAction(action);
    setIsGlobalPin(false);
    setMoveBoardId(action === "move" ? eligibleBoards[0]?.id ?? "" : "");
  }

  const copy = selectedAction ? threadActionCopy[selectedAction] : threadActionCopy.hide;

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button type="button" variant="outline" aria-label="管理此帖子">
            <ShieldCheck className="size-4" aria-hidden="true" />
            管理
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-52">
          <DropdownMenuLabel>帖子治理</DropdownMenuLabel>
          {actions.map((action) => (
            <DropdownMenuItem
              key={action}
              variant={threadActionCopy[action].destructive ? "destructive" : "default"}
              onSelect={() => selectAction(action)}
            >
              <ThreadActionIcon action={action} />
              {threadActionLabel(action, thread.pinnedGlobally ?? false)}
            </DropdownMenuItem>
          ))}
          <StaffTargetLinks authorHandle={thread.authorHandle} />
        </DropdownMenuContent>
      </DropdownMenu>

      <ReasonDialog
        open={Boolean(selectedAction)}
        onOpenChange={(open) => {
          if (!open && !mutation.isPending) {
            setSelectedAction(null);
          }
        }}
        title={copy.title}
        description={copy.description}
        confirmLabel={copy.confirmLabel}
        destructive={copy.destructive}
        isPending={mutation.isPending}
        confirmDisabled={selectedAction === "move" && !moveBoardId}
        onConfirm={(reason) => {
          if (!selectedAction) {
            return;
          }
          mutation.mutate({
            action: selectedAction,
            reason,
            boardId: selectedAction === "move" ? moveBoardId : undefined,
            globally: selectedAction === "pin" ? isGlobalPin : undefined,
          });
        }}
      >
        {selectedAction === "move" ? (
          <div className="space-y-2">
            <Label htmlFor="thread-moderation-board">目标板块</Label>
            <Select value={moveBoardId} onValueChange={setMoveBoardId}>
              <SelectTrigger id="thread-moderation-board">
                <SelectValue placeholder="选择目标板块" />
              </SelectTrigger>
              <SelectContent>
                {eligibleBoards.map((board) => (
                  <SelectItem key={board.id} value={board.id ?? ""}>
                    {board.name ?? board.slug ?? `板块 ${board.id}`}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ) : null}
        {selectedAction === "pin" ? (
          <div className="flex items-center justify-between gap-4 rounded-lg border p-3">
            <div className="space-y-1">
              <Label htmlFor="thread-moderation-global-pin">全站置顶</Label>
              <p className="text-xs text-muted-foreground">仅在确需覆盖所有板块时启用。</p>
            </div>
            <Switch
              id="thread-moderation-global-pin"
              checked={isGlobalPin}
              onCheckedChange={setIsGlobalPin}
            />
          </div>
        ) : null}
      </ReasonDialog>
    </>
  );
}

export function ThreadModerationControls({
  thread,
  boards,
}: {
  thread: ThreadDetailWithPoll;
  boards: Board[];
}) {
  return thread.canModerate && thread.id ? (
    <ThreadModerationMenu thread={thread} boards={boards} />
  ) : null;
}

export function CommentModerationMenu({ comment, threadId }: { comment: Comment; threadId: string }) {
  const queryClient = useQueryClient();
  const [selectedAction, setSelectedAction] = React.useState<CommentModerationAction | null>(null);
  const commentId = comment.id ?? "";
  const actions: CommentModerationAction[] = comment.isDeleted
    ? ["restore"]
    : [comment.isHidden ? "unhide" : "hide", "delete"];

  const mutation = useMutation({
    mutationFn: ({ action, reason }: { action: CommentModerationAction; reason: string }) =>
      api.moderateForumComment(commentId, action, reason),
    onSuccess: async (_response, variables) => {
      toast.success(`${commentActionCopy[variables.action].title}已完成`);
      setSelectedAction(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.thread(threadId) }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.comments(threadId) }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.feeds() }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.homeFeeds() }),
        queryClient.invalidateQueries({ queryKey: forumQueryKeys.profile(comment.authorHandle) }),
        queryClient.invalidateQueries({ queryKey: ["admin", "forum-flags"] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "forum", "comment", commentId] }),
        queryClient.invalidateQueries({ queryKey: ["admin", "overview"] }),
      ]);
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : "管理操作失败"),
  });

  const copy = selectedAction ? commentActionCopy[selectedAction] : commentActionCopy.hide;

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button type="button" size="icon" variant="ghost" aria-label="管理此回复">
            <MoreHorizontal className="size-4" aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-52">
          <DropdownMenuLabel>回复治理</DropdownMenuLabel>
          {actions.map((action) => (
            <DropdownMenuItem
              key={action}
              variant={commentActionCopy[action].destructive ? "destructive" : "default"}
              onSelect={() => setSelectedAction(action)}
            >
              {action === "delete" ? <Trash2 className="size-4" aria-hidden="true" /> : null}
              {action === "restore" ? <RotateCcw className="size-4" aria-hidden="true" /> : null}
              {action === "hide" ? <EyeOff className="size-4" aria-hidden="true" /> : null}
              {action === "unhide" ? <Eye className="size-4" aria-hidden="true" /> : null}
              {commentActionCopy[action].title}
            </DropdownMenuItem>
          ))}
          <StaffTargetLinks authorHandle={comment.authorHandle} />
        </DropdownMenuContent>
      </DropdownMenu>

      <ReasonDialog
        open={Boolean(selectedAction)}
        onOpenChange={(open) => {
          if (!open && !mutation.isPending) {
            setSelectedAction(null);
          }
        }}
        title={copy.title}
        description={copy.description}
        confirmLabel={copy.confirmLabel}
        destructive={copy.destructive}
        isPending={mutation.isPending}
        onConfirm={(reason) => {
          if (selectedAction) {
            mutation.mutate({ action: selectedAction, reason });
          }
        }}
      />
    </>
  );
}

export function CommentModerationControls({
  comment,
  threadId,
}: {
  comment: Comment;
  threadId: string;
}) {
  return comment.canModerate && comment.id ? (
    <CommentModerationMenu comment={comment} threadId={threadId} />
  ) : null;
}

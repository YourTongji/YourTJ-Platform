import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import type { DmConversation, DmMessage } from "@/lib/api/types";
import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ConversationThread } from "./conversation-thread";

const requestConversation: DmConversation = {
  id: "42",
  participantId: "2",
  participantHandle: "requester",
  participantAvatarUrl: null,
  lastMessageExcerpt: "想请教课程资料",
  lastMessageAt: 1_700_000_000,
  unreadCount: 0,
  isArchived: false,
  isMuted: false,
  isDeleted: false,
  requestStatus: "pending",
  requestDirection: "incoming",
  canSend: false,
  createdAt: 1_700_000_000,
};

const requestMessage: DmMessage = {
  id: "9",
  conversationId: "42",
  senderId: "2",
  senderHandle: "requester",
  body: "想请教课程资料",
  createdAt: 1_700_000_000,
};

describe("ConversationThread message requests", () => {
  it("keeps delivery locked and exposes explicit accept, delete, and report actions", async () => {
    const user = userEvent.setup();
    const onAcceptRequest = vi.fn();
    const onDeclineRequest = vi.fn();
    const onReport = vi.fn();
    const view = render(
      <MemoryRouter>
        <ConversationThread
          conversation={requestConversation}
          messages={[requestMessage]}
          currentAccountId="1"
          body=""
          isIgnored={false}
          relationshipPending={false}
          lifecyclePending={false}
          requestActionPending={false}
          isLoading={false}
          isSending={false}
          hasOlder={false}
          isLoadingOlder={false}
          onBodyChange={vi.fn()}
          onBack={vi.fn()}
          onRetry={vi.fn()}
          onLoadOlder={vi.fn()}
          onSend={vi.fn()}
          onReport={onReport}
          onAcceptRequest={onAcceptRequest}
          onDeclineRequest={onDeclineRequest}
          onToggleIgnore={vi.fn()}
          onToggleArchive={vi.fn()}
          onToggleMute={vi.fn()}
          onDelete={vi.fn()}
        />
      </MemoryRouter>,
    );

    expect(screen.queryByRole("textbox", { name: "消息内容" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "接受" }));
    expect(onAcceptRequest).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: "删除请求" }));
    expect(onDeclineRequest).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: "举报" }));
    expect(onReport).toHaveBeenCalledWith(requestMessage, true);
    await expectNoAccessibilityViolations(view.container);
  });
});

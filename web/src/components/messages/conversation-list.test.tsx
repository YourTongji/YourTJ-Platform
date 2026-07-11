import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { DmConversation } from "@/lib/api/types";
import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { ConversationList } from "./conversation-list";

const conversation: DmConversation = {
  id: "12",
  participantId: "2",
  participantHandle: "alice",
  participantAvatarUrl: null,
  lastMessageExcerpt: "课程资料已经发你了",
  lastMessageAt: 1_700_000_000,
  unreadCount: 0,
  isArchived: false,
  isMuted: true,
  isDeleted: true,
  createdAt: 1_699_000_000,
};

describe("ConversationList", () => {
  it("keeps participant deletion recoverable and labels muted state", async () => {
    const user = userEvent.setup();
    const onRecover = vi.fn();
    const view = render(
      <ConversationList
        conversations={[conversation]}
        selectedId=""
        view="deleted"
        searchQuery=""
        headerAction={null}
        isLoading={false}
        hasMore={false}
        isLoadingMore={false}
        isRecovering={false}
        onRetry={vi.fn()}
        onLoadMore={vi.fn()}
        onSelect={vi.fn()}
        onViewChange={vi.fn()}
        onSearchChange={vi.fn()}
        onRecover={onRecover}
      />,
    );

    expect(screen.getByLabelText("已静音")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "恢复与 alice 的会话" }));
    expect(onRecover).toHaveBeenCalledWith(conversation);
    await expectNoAccessibilityViolations(view.container);
  });
});

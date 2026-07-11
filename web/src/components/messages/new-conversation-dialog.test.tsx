import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { NewConversationDialog } from "./new-conversation-dialog";

describe("NewConversationDialog", () => {
  it("requires a bounded request introduction and submits an idempotency key", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue({
      id: "1",
      participantHandle: "recipient",
      requestStatus: "pending",
    });
    const onDismiss = vi.fn();
    const view = render(
      <NewConversationDialog
        canCreate
        isPending={false}
        onReset={vi.fn()}
        onDismiss={onDismiss}
        onCreate={onCreate}
      />,
    );

    await user.click(screen.getByRole("button", { name: "新建私信" }));
    const submit = screen.getByRole("button", { name: "发送并开始" });
    expect(submit).toBeDisabled();
    await user.type(screen.getByLabelText("对方 handle"), "recipient");
    await user.type(screen.getByLabelText("请求附言"), "想请教课程资料");
    await expectNoAccessibilityViolations(view.baseElement);
    await user.click(submit);

    expect(onCreate).toHaveBeenCalledWith(
      "recipient",
      "想请教课程资料",
      expect.stringMatching(/^dm-request:/),
    );
    expect(onDismiss).not.toHaveBeenCalled();
  });
});

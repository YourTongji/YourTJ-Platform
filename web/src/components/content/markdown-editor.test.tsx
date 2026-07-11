import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { MarkdownEditor } from "./markdown-editor";

describe("MarkdownEditor", () => {
  it("uses the same safe renderer for its preview", async () => {
    const user = userEvent.setup();
    const view = render(
      <MarkdownEditor
        value="**可预览内容**"
        onChange={vi.fn()}
        label="主题正文"
        maxLength={50_000}
      />,
    );

    expect(screen.getByLabelText("主题正文")).toBeVisible();
    await user.click(screen.getByRole("tab", { name: "预览" }));
    expect(screen.getByText("可预览内容").tagName).toBe("STRONG");
    await expectNoAccessibilityViolations(view.container);
  });
});

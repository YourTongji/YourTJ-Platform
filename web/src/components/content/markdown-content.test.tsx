import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { MarkdownContent } from "./markdown-content";

describe("MarkdownContent", () => {
  it("renders GFM while removing raw HTML, dangerous links, and remote images", async () => {
    const view = render(
      <MarkdownContent
        format="markdown_v1"
        content={'**加粗**\n\n<script>alert(1)</script>\n\n[危险链接](javascript:alert(1))\n\n[安全外链](https://example.com/path)\n\n![头像](https://tracker.example/pixel.png)'}
      />,
    );

    expect(screen.getByText("加粗").tagName).toBe("STRONG");
    expect(view.container.querySelector("script")).not.toBeInTheDocument();
    expect(screen.getByText("危险链接")).not.toHaveAttribute("href");
    expect(screen.getByRole("link", { name: "安全外链" })).toHaveAttribute("rel", "noopener noreferrer nofollow ugc");
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(screen.getByText("图片尚未通过平台资产校验：头像")).toBeVisible();
    await expectNoAccessibilityViolations(view.container);
  });

  it("never guesses that legacy plain text is Markdown", () => {
    const view = render(<MarkdownContent format="plain_v1" content="**仍是纯文本**" />);
    expect(view.container.querySelector("strong")).not.toBeInTheDocument();
    expect(screen.getByText("**仍是纯文本**")).toBeVisible();
  });
});

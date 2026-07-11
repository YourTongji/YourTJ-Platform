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
    expect(screen.getByRole("img", { name: "图片不可用：头像" })).toBeVisible();
    expect(screen.getByText("图片当前不可用：头像")).toBeVisible();
    await expectNoAccessibilityViolations(view.container);
  });

  it("maps a vendor reference only to the matching server-derived clean attachment URL", () => {
    render(
      <MarkdownContent
        format="markdown_v1"
        content="![校园风景](yourtj-asset:42)\n\n![缺失](yourtj-asset:43)\n\n![**校园夜景**](yourtj-asset:44)"
        attachments={[
          {
            assetId: "42",
            reference: "yourtj-asset:42",
            position: 0,
            alt: "校园风景",
            url: "https://cdn.example.test/derived-42.webp",
            width: 1200,
            height: 800,
          },
          {
            assetId: "44",
            reference: "yourtj-asset:44",
            position: 2,
            alt: "校园夜景",
            url: "https://cdn.example.test/derived-44.webp",
            width: null,
            height: null,
          },
        ]}
      />,
    );

    expect(screen.getByRole("img", { name: "校园风景" })).toHaveAttribute(
      "src",
      "https://cdn.example.test/derived-42.webp",
    );
    expect(screen.getByRole("img", { name: "图片不可用：缺失" })).toBeVisible();
    expect(screen.getByRole("img", { name: "校园夜景" })).toHaveAttribute(
      "src",
      "https://cdn.example.test/derived-44.webp",
    );
    expect(document.querySelector('img[src^="yourtj-asset:"]')).not.toBeInTheDocument();
  });

  it("never guesses that legacy plain text is Markdown", () => {
    const view = render(<MarkdownContent format="plain_v1" content="**仍是纯文本**" />);
    expect(view.container.querySelector("strong")).not.toBeInTheDocument();
    expect(screen.getByText("**仍是纯文本**")).toBeVisible();
  });
});

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { expectNoAccessibilityViolations } from "@/test/accessibility";

import { LightboxableImage } from "./image-lightbox";

const images = [
  { src: "https://media.example.test/one.webp", alt: "第一张", width: 800, height: 600 },
  { src: "https://media.example.test/two.webp", alt: "第二张", width: 600, height: 800 },
];

describe("LightboxableImage", () => {
  it("opens an accessible gallery, supports arrow navigation, and restores trigger focus", async () => {
    const user = userEvent.setup();
    const view = render(
      <LightboxableImage
        src={images[0].src}
        alt={images[0].alt}
        width={images[0].width}
        height={images[0].height}
        images={images}
      />,
    );
    const trigger = screen.getByRole("button", { name: "查看大图：第一张" });

    await user.click(trigger);
    const dialog = screen.getByRole("dialog", { name: /第一张/ });
    expect(within(dialog).getByRole("img", { name: "第一张" })).toBeVisible();
    expect(within(dialog).getByText("1/2 · 100%")).toBeVisible();
    await user.click(within(dialog).getByRole("button", { name: "放大图片" }));
    expect(within(dialog).getByText("1/2 · 150%")).toBeVisible();
    expect(within(dialog).getByRole("link", { name: "下载原图" })).toHaveAttribute(
      "href",
      images[0].src,
    );
    await user.keyboard("{ArrowRight}");
    expect(within(dialog).getByRole("img", { name: "第二张" })).toBeVisible();
    expect(within(dialog).getByText("2/2 · 100%")).toBeVisible();
    await expectNoAccessibilityViolations(view.container);

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(trigger).toHaveFocus();
  });

  it("closes when the user clicks empty lightbox space", async () => {
    const user = userEvent.setup();
    render(<LightboxableImage src={images[0].src} alt={images[0].alt} />);

    await user.click(screen.getByRole("button", { name: "查看大图：第一张" }));
    await user.click(screen.getByRole("dialog", { name: "第一张" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});

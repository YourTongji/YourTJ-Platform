import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ForumDeliveryImage } from "./forum-delivery-image";

const attachment = {
  assetId: "42",
  reference: "yourtj-asset:42",
  position: 0,
  alt: "校园风景",
  url: "https://media.example.test/42.webp",
  expiresAt: Math.floor(Date.now() / 1000) + 300,
  width: 1280,
  height: 720,
};

describe("ForumDeliveryImage", () => {
  it("lazy-loads content media and requests bounded recovery after a delivery failure", () => {
    const onDeliveryRefresh = vi.fn();
    render(
      <ForumDeliveryImage
        attachment={attachment}
        onDeliveryRefresh={onDeliveryRefresh}
      />,
    );

    const image = screen.getByRole("img", { name: "校园风景" });
    expect(image).toHaveAttribute("loading", "lazy");
    expect(image).toHaveAttribute("decoding", "async");
    fireEvent.error(image);
    fireEvent.error(image);
    expect(onDeliveryRefresh).toHaveBeenCalledOnce();
  });
});

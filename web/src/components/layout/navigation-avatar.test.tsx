import { act, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { NavigationAvatar } from "./navigation-avatar";

vi.mock("@/components/ui/avatar", () => ({
  Avatar: ({ children, ...props }: ComponentProps<"span">) => <span {...props}>{children}</span>,
  AvatarImage: ({ onLoadingStatusChange, ...props }: ComponentProps<"img"> & {
    onLoadingStatusChange?: (status: string) => void;
  }) => {
    void onLoadingStatusChange;
    return <img {...props} />;
  },
  AvatarFallback: ({ children, ...props }: ComponentProps<"span">) => (
    <span {...props}>{children}</span>
  ),
}));

class PendingImage {
  static instances: PendingImage[] = [];

  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  referrerPolicy = "";
  src = "";

  constructor() {
    PendingImage.instances.push(this);
  }
}

describe("NavigationAvatar", () => {
  afterEach(() => {
    PendingImage.instances = [];
    vi.unstubAllGlobals();
  });

  it("uses a neutral cold-start fallback and keeps the last loaded avatar during refresh", () => {
    vi.stubGlobal("Image", PendingImage);
    const firstDelivery = {
      assetId: "7",
      variant: "thumb_256" as const,
      url: "https://media.example.test/first.webp",
      expiresAt: 1_900_000_000,
      mime: "image/webp" as const,
      width: 256,
      height: 256,
    };
    const secondDelivery = { ...firstDelivery, url: "https://media.example.test/second.webp" };
    const view = render(
      <NavigationAvatar
        handle="alice"
        isResolving
        onDeliveryError={vi.fn()}
      />,
    );

    expect(screen.getByRole("img", { name: "头像加载中" })).toBeEmptyDOMElement();
    expect(screen.queryByText("A")).not.toBeInTheDocument();

    view.rerender(
      <NavigationAvatar
        delivery={firstDelivery}
        handle="alice"
        isResolving={false}
        onDeliveryError={vi.fn()}
      />,
    );
    act(() => PendingImage.instances[0].onload?.());
    expect(screen.getByRole("img", { name: "alice 的头像" })).toHaveAttribute(
      "src",
      firstDelivery.url,
    );

    view.rerender(
      <NavigationAvatar
        delivery={secondDelivery}
        handle="alice"
        isResolving={false}
        onDeliveryError={vi.fn()}
      />,
    );
    expect(screen.getByRole("img", { name: "alice 的头像" })).toHaveAttribute(
      "src",
      firstDelivery.url,
    );
    act(() => PendingImage.instances[1].onload?.());
    expect(screen.getByRole("img", { name: "alice 的头像" })).toHaveAttribute(
      "src",
      secondDelivery.url,
    );
  });

  it("keeps the last loaded avatar without retrying a failed replacement URL", () => {
    vi.stubGlobal("Image", PendingImage);
    const firstDelivery = {
      assetId: "7",
      variant: "thumb_256" as const,
      url: "https://media.example.test/first.webp",
      expiresAt: 1_900_000_000,
      mime: "image/webp" as const,
      width: 256,
      height: 256,
    };
    const secondDelivery = { ...firstDelivery, url: "https://media.example.test/second.webp" };
    const onDeliveryError = vi.fn();
    const view = render(
      <NavigationAvatar
        delivery={firstDelivery}
        handle="alice"
        isResolving={false}
        onDeliveryError={onDeliveryError}
      />,
    );

    act(() => PendingImage.instances[0].onload?.());
    view.rerender(
      <NavigationAvatar
        delivery={secondDelivery}
        handle="alice"
        isResolving={false}
        onDeliveryError={onDeliveryError}
      />,
    );
    act(() => PendingImage.instances[1].onerror?.());

    expect(screen.getByRole("img", { name: "alice 的头像" })).toHaveAttribute(
      "src",
      firstDelivery.url,
    );
    expect(onDeliveryError).toHaveBeenCalledOnce();
    expect(PendingImage.instances).toHaveLength(2);
  });
});

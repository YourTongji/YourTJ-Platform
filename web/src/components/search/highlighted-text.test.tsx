import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { HighlightedText } from "./highlighted-text";

describe("HighlightedText", () => {
  it("uses Unicode character offsets and renders canonical text without HTML injection", () => {
    const view = render(
      <p>
        <HighlightedText
          text={'算法 <img src=x onerror="alert(1)"> Algorithm'}
          ranges={[{ start: 0, end: 2 }]}
        />
      </p>,
    );

    expect(screen.getByText("算法", { selector: "mark" })).toBeVisible();
    expect(view.container.querySelector("img")).toBeNull();
    expect(view.container).toHaveTextContent('<img src=x onerror="alert(1)">');
  });

  it("ignores invalid and overlapping server ranges", () => {
    const view = render(
      <HighlightedText text="Algorithm" ranges={[
        { start: -1, end: 2 },
        { start: 0, end: 5 },
        { start: 3, end: 8 },
        { start: 20, end: 30 },
      ]} />,
    );

    expect(view.container.querySelectorAll("mark")).toHaveLength(1);
    expect(view.container.querySelector("mark")).toHaveTextContent("Algor");
  });
});

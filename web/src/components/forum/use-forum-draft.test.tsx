import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "@/lib/api/client";
import type { DraftPayload } from "@/lib/api/types";

import { useForumDraft } from "./use-forum-draft";

const apiMocks = vi.hoisted(() => ({
  draft: vi.fn(),
  saveDraft: vi.fn(),
  deleteDraft: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));

type CommentDraft = Extract<DraftPayload, { kind: "comment" }>;

const emptyDraft: CommentDraft = {
  kind: "comment",
  threadId: "42",
  body: "",
  contentFormat: "markdown_v1",
  parentId: null,
};

describe("useForumDraft", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    apiMocks.draft.mockReset().mockRejectedValue(new ApiError(404, "not found"));
    apiMocks.saveDraft.mockReset().mockResolvedValue({
      draftKey: "comment:42",
      payload: { ...emptyDraft, body: "new reply" },
      version: 1,
      updatedAt: 1_700_000_000,
    });
    apiMocks.deleteDraft.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => vi.useRealTimers());

  it("debounces the first save with expectedVersion zero", async () => {
    const onRestore = vi.fn();
    const view = renderHook(
      ({ payload }: { payload: CommentDraft }) => useForumDraft({
        draftKey: "comment:42",
        enabled: true,
        isEmpty: payload.body.length === 0,
        payload,
        onRestore,
      }),
      { initialProps: { payload: emptyDraft } },
    );

    await act(async () => Promise.resolve());
    expect(view.result.current.status).toBe("idle");
    expect(apiMocks.saveDraft).not.toHaveBeenCalled();

    const updated = { ...emptyDraft, body: "new reply" };
    view.rerender({ payload: updated });
    await act(async () => vi.advanceTimersByTimeAsync(899));
    expect(apiMocks.saveDraft).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTimeAsync(1));

    expect(apiMocks.saveDraft).toHaveBeenCalledWith({
      draftKey: "comment:42",
      expectedVersion: 0,
      payload: updated,
    });
    expect(view.result.current.status).toBe("saved");
    expect(onRestore).not.toHaveBeenCalled();
  });
});

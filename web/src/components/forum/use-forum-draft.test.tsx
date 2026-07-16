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

const localDraftMocks = vi.hoisted(() => ({
  isAvailable: vi.fn(),
  read: vi.fn(),
  write: vi.fn(),
  delete: vi.fn(),
}));

vi.mock("@/lib/api/endpoints", () => ({ api: apiMocks }));
vi.mock("@/lib/local-forum-drafts", () => ({
  isLocalForumDraftStorageAvailable: localDraftMocks.isAvailable,
  readLocalForumDraft: localDraftMocks.read,
  writeLocalForumDraft: localDraftMocks.write,
  deleteLocalForumDraft: localDraftMocks.delete,
}));

type CommentDraft = Extract<DraftPayload, { kind: "comment" }>;

const emptyDraft: CommentDraft = {
  kind: "comment",
  threadId: "42",
  body: "",
  contentFormat: "markdown_v1",
  parentId: null,
  attachmentAssetIds: [],
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
    localDraftMocks.isAvailable.mockReset().mockReturnValue(true);
    localDraftMocks.read.mockReset().mockResolvedValue(null);
    localDraftMocks.write.mockReset().mockResolvedValue(undefined);
    localDraftMocks.delete.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => vi.useRealTimers());

  it("debounces the first save with expectedVersion zero", async () => {
    const onRestore = vi.fn();
    const view = renderHook(
      ({ payload }: { payload: CommentDraft }) => useForumDraft({
        draftKey: "comment:42",
        accountId: "account-1",
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

  it("restores a newer local recovery copy and lets the user choose the cloud version", async () => {
    const cloudPayload = { ...emptyDraft, body: "cloud reply" };
    const localPayload = { ...emptyDraft, body: "local reply" };
    apiMocks.draft.mockResolvedValue({
      draftKey: "comment:42",
      payload: cloudPayload,
      version: 3,
      updatedAt: 1_700_000_000,
    });
    localDraftMocks.read.mockResolvedValue({
      accountId: "account-1",
      draftKey: "comment:42",
      payload: localPayload,
      updatedAt: 1_700_000_100,
      expiresAt: 1_700_600_000,
    });
    const onRestore = vi.fn();
    const view = renderHook(() => useForumDraft({
      draftKey: "comment:42",
      accountId: "account-1",
      enabled: true,
      isEmpty: true,
      payload: emptyDraft,
      onRestore,
    }));

    await act(async () => Promise.resolve());
    expect(view.result.current.status).toBe("conflict");
    expect(onRestore).toHaveBeenLastCalledWith(localPayload);

    await act(async () => {
      view.result.current.restoreRemote();
      await Promise.resolve();
    });

    expect(onRestore).toHaveBeenLastCalledWith(cloudPayload);
    expect(view.result.current.status).toBe("saved");
    expect(localDraftMocks.write).toHaveBeenCalledWith(
      "account-1",
      "comment:42",
      cloudPayload,
      1_700_000_000,
    );
  });

  it("backs up by account and removes both copies when the draft is cleared", async () => {
    const view = renderHook(
      ({ payload }: { payload: CommentDraft }) => useForumDraft({
        draftKey: "comment:42",
        accountId: "account-1",
        enabled: true,
        isEmpty: payload.body.length === 0,
        payload,
        onRestore: vi.fn(),
      }),
      { initialProps: { payload: emptyDraft } },
    );
    await act(async () => Promise.resolve());
    expect(view.result.current.status).toBe("idle");

    const updated = { ...emptyDraft, body: "recover me" };
    view.rerender({ payload: updated });
    await act(async () => vi.advanceTimersByTimeAsync(350));
    expect(localDraftMocks.write).toHaveBeenCalledWith(
      "account-1",
      "comment:42",
      updated,
    );

    await act(async () => view.result.current.clearDraft());

    expect(apiMocks.deleteDraft).toHaveBeenCalledWith("comment:42");
    expect(localDraftMocks.delete).toHaveBeenCalledWith("account-1", "comment:42");
    expect(view.result.current.localBackupStatus).toBe("idle");
  });
});

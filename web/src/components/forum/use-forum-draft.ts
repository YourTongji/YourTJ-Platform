import * as React from "react";

import { ApiError } from "@/lib/api/client";
import { api } from "@/lib/api/endpoints";
import type { Draft, DraftPayload } from "@/lib/api/types";

export type DraftSyncStatus = "disabled" | "loading" | "idle" | "saving" | "saved" | "conflict" | "error";

interface ForumDraftOptions<TPayload extends DraftPayload> {
  draftKey: string;
  enabled: boolean;
  isEmpty: boolean;
  payload: TPayload;
  onRestore: (payload: TPayload) => void;
}

export function useForumDraft<TPayload extends DraftPayload>({
  draftKey,
  enabled,
  isEmpty,
  payload,
  onRestore,
}: ForumDraftOptions<TPayload>) {
  const [status, setStatus] = React.useState<DraftSyncStatus>(enabled ? "loading" : "disabled");
  const [isLoaded, setIsLoaded] = React.useState(false);
  const [savedAt, setSavedAt] = React.useState<number | null>(null);
  const [remoteConflict, setRemoteConflict] = React.useState<Draft | null>(null);
  const serializedPayload = JSON.stringify(payload);
  const versionRef = React.useRef(0);
  const generationRef = React.useRef(0);
  const inFlightRef = React.useRef(false);
  const inFlightPromiseRef = React.useRef<Promise<void> | null>(null);
  const dirtyRef = React.useRef(false);
  const timerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const payloadRef = React.useRef<TPayload>(payload);
  const serializedRef = React.useRef(serializedPayload);
  const lastSavedRef = React.useRef("");
  const isEmptyRef = React.useRef(isEmpty);
  const onRestoreRef = React.useRef(onRestore);
  const flushRef = React.useRef<() => void>(() => undefined);

  payloadRef.current = payload;
  serializedRef.current = serializedPayload;
  isEmptyRef.current = isEmpty;
  onRestoreRef.current = onRestore;

  flushRef.current = () => {
    if (!enabled || !isLoaded || inFlightRef.current || remoteConflict) return;
    const serialized = serializedRef.current;
    dirtyRef.current = serialized !== lastSavedRef.current;
    if (!dirtyRef.current || (versionRef.current === 0 && isEmptyRef.current)) {
      setStatus(versionRef.current > 0 ? "saved" : "idle");
      return;
    }

    const generation = generationRef.current;
    const expectedVersion = versionRef.current;
    const submittedPayload = payloadRef.current;
    dirtyRef.current = false;
    inFlightRef.current = true;
    setStatus("saving");
    const savePromise = api
      .saveDraft({ draftKey, expectedVersion, payload: submittedPayload })
      .then((draft) => {
        if (generation !== generationRef.current) return;
        versionRef.current = draft.version;
        lastSavedRef.current = serialized;
        setSavedAt(draft.updatedAt);
        setStatus("saved");
      })
      .catch(async (error: unknown) => {
        if (generation !== generationRef.current) return;
        if (error instanceof ApiError && error.status === 409) {
          try {
            const remote = await api.draft(draftKey);
            if (generation === generationRef.current) {
              setRemoteConflict(remote);
              setStatus("conflict");
            }
          } catch {
            if (generation === generationRef.current) setStatus("error");
          }
          return;
        }
        setStatus("error");
      })
      .finally(() => {
        if (generation !== generationRef.current) return;
        inFlightRef.current = false;
        inFlightPromiseRef.current = null;
        if (serializedRef.current !== lastSavedRef.current && !remoteConflict) {
          if (timerRef.current) clearTimeout(timerRef.current);
          timerRef.current = setTimeout(() => flushRef.current(), 500);
        }
      });
    inFlightPromiseRef.current = savePromise;
  };

  React.useEffect(() => {
    generationRef.current += 1;
    const generation = generationRef.current;
    if (timerRef.current) clearTimeout(timerRef.current);
    inFlightRef.current = false;
    versionRef.current = 0;
    lastSavedRef.current = "";
    setRemoteConflict(null);
    setSavedAt(null);
    setIsLoaded(false);

    if (!enabled) {
      setStatus("disabled");
      return;
    }

    setStatus("loading");
    void api
      .draft(draftKey)
      .then((draft) => {
        if (generation !== generationRef.current) return;
        if (draft.payload.kind !== payloadRef.current.kind) {
          setStatus("error");
          setIsLoaded(true);
          return;
        }
        versionRef.current = draft.version;
        lastSavedRef.current = JSON.stringify(draft.payload);
        setSavedAt(draft.updatedAt);
        onRestoreRef.current(draft.payload as TPayload);
        setStatus("saved");
        setIsLoaded(true);
      })
      .catch((error: unknown) => {
        if (generation !== generationRef.current) return;
        if (error instanceof ApiError && error.status === 404) {
          setStatus("idle");
        } else {
          setStatus("error");
        }
        setIsLoaded(true);
      });

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [draftKey, enabled]);

  React.useEffect(() => {
    if (!enabled || !isLoaded || remoteConflict) return;
    dirtyRef.current = serializedRef.current !== lastSavedRef.current;
    if (!dirtyRef.current || (versionRef.current === 0 && isEmpty)) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => flushRef.current(), 900);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [enabled, isEmpty, isLoaded, remoteConflict, serializedPayload]);

  const restoreRemote = React.useCallback(() => {
    if (!remoteConflict) return;
    versionRef.current = remoteConflict.version;
    lastSavedRef.current = JSON.stringify(remoteConflict.payload);
    setSavedAt(remoteConflict.updatedAt);
    onRestoreRef.current(remoteConflict.payload as TPayload);
    setRemoteConflict(null);
    setStatus("saved");
  }, [remoteConflict]);

  const keepLocal = React.useCallback(() => {
    if (!remoteConflict) return;
    versionRef.current = remoteConflict.version;
    lastSavedRef.current = JSON.stringify(remoteConflict.payload);
    setRemoteConflict(null);
    setStatus("idle");
    timerRef.current = setTimeout(() => flushRef.current(), 0);
  }, [remoteConflict]);

  const retry = React.useCallback(() => {
    setStatus("idle");
    timerRef.current = setTimeout(() => flushRef.current(), 0);
  }, []);

  const saveNow = React.useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    flushRef.current();
  }, []);

  const clearDraft = React.useCallback(async () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    await inFlightPromiseRef.current;
    generationRef.current += 1;
    await api.deleteDraft(draftKey);
    versionRef.current = 0;
    lastSavedRef.current = serializedRef.current;
    setSavedAt(null);
    setRemoteConflict(null);
    setStatus("idle");
  }, [draftKey]);

  return {
    status,
    savedAt,
    remoteConflict,
    restoreRemote,
    keepLocal,
    retry,
    saveNow,
    clearDraft,
  };
}

import { describe, expect, it } from "vitest";

import { accountQueryKeys } from "./account-query-keys";

describe("account query keys", () => {
  it("partitions notification data by account", () => {
    expect(accountQueryKeys.notifications("account-a"))
      .not.toEqual(accountQueryKeys.notifications("account-b"));
  });
});

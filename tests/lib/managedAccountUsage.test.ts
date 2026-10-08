import { describe, expect, it } from "vitest";
import type { Provider } from "@/types";
import {
  findManagedAccountUsers,
  groupUsersByApp,
} from "@/lib/managedAccountUsage";

const provider = (
  id: string,
  extra: Partial<Provider> = {},
  settingsConfig: Record<string, unknown> = {},
): Provider =>
  ({
    id,
    name: id,
    settingsConfig,
    ...extra,
  }) as Provider;

const byId = (...list: Provider[]) =>
  Object.fromEntries(list.map((p) => [p.id, p]));

const boundTo = (accountId: string): Partial<Provider> => ({
  category: "official",
  meta: {
    providerType: "codex_oauth",
    authBinding: {
      source: "managed_account",
      authProvider: "codex_oauth",
      accountId,
    },
  },
});

describe("findManagedAccountUsers", () => {
  const codex = byId(
    // 没绑定账号的官方卡用 Codex 自己的登录，不算在用托管账号
    provider(
      "codex-official",
      { category: "official", meta: { providerType: "codex_oauth" } },
      { auth: {}, config: "" },
    ),
    provider("bound-c1", boundTo("c1"), { auth: {}, config: "" }),
    provider("bound-c2", boundTo("c2"), { auth: {}, config: "" }),
    // 普通第三方供应商
    provider(
      "plain",
      {},
      { auth: { OPENAI_API_KEY: "sk-test" }, config: "" },
    ),
  );

  it("counts only the providers bound to the removed account", () => {
    const users = findManagedAccountUsers("codex_oauth", ["c1"], { codex });
    expect(users.map((u) => [u.appId, u.providerId])).toEqual([
      ["codex", "bound-c1"],
    ]);
  });

  it("collects every bound provider when removing all accounts", () => {
    const users = findManagedAccountUsers("codex_oauth", ["c1", "c2"], {
      codex,
    });
    expect(users.map((u) => u.providerId)).toEqual(["bound-c1", "bound-c2"]);
    expect(groupUsersByApp(users)).toEqual([
      { appId: "codex", names: ["bound-c1", "bound-c2"] },
    ]);
  });

  it("finds nothing when no provider list is loaded", () => {
    expect(findManagedAccountUsers("codex_oauth", ["c1"], {})).toEqual([]);
  });
});

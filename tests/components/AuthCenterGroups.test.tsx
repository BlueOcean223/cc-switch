import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { TFunction } from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";

const mocks = vi.hoisted(() => ({
  useCodexOauth: vi.fn(),
}));

vi.mock("@/components/providers/forms/hooks/useCodexOauth", () => ({
  useCodexOauth: mocks.useCodexOauth,
}));
// 「N 个供应商在用」要读供应商列表（React Query）；这里不关心，给空
vi.mock("@/components/providers/forms/hooks/useManagedAccountUsers", () => ({
  useManagedAccountUsers: () => () => [],
}));

const baseAuth = () => ({
  accounts: [],
  isStatusSuccess: true,
  isStatusError: false,
  hasAnyAccount: false,
  isAuthenticated: false,
  pollingState: "idle" as const,
  deviceCode: null,
  error: null,
  isPolling: false,
  isAddingAccount: false,
  isRemovingAccount: false,
  addAccount: vi.fn(),
  reauthAccount: vi.fn(),
  retryAuth: vi.fn(),
  removeAccount: vi.fn(),
  cancelAuth: vi.fn(),
  logout: vi.fn(),
  refetchStatus: vi.fn(),
});

describe("Auth Center account group", () => {
  beforeEach(() => {
    mocks.useCodexOauth.mockReturnValue(baseAuth());
  });

  it("offers a retry when the account status fails to load", async () => {
    const user = userEvent.setup();
    const auth = { ...baseAuth(), isStatusSuccess: false, isStatusError: true };
    mocks.useCodexOauth.mockReturnValue(auth);
    render(<CodexOAuthSection />);

    expect(
      screen.getByText("无法加载 ChatGPT 账号状态，请重试。"),
    ).toBeInTheDocument();
    // 页面一打开就在的提示不用 role=alert
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重试" }));
    expect(auth.refetchStatus).toHaveBeenCalledTimes(1);
  });

  it("shows the empty state with a sign-in button", async () => {
    const user = userEvent.setup();
    const auth = baseAuth();
    mocks.useCodexOauth.mockReturnValue(auth);
    render(<CodexOAuthSection showAccountQuota />);

    expect(screen.getByText("还没有登录 ChatGPT 账号。")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "添加账号" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /使用 ChatGPT 登录/ }));
    expect(auth.addAccount).toHaveBeenCalledTimes(1);
  });
});

describe("subscriptionQuotaState", () => {
  const t = ((key: string, opts?: { defaultValue?: string }) =>
    opts?.defaultValue ?? key) as unknown as TFunction;

  it("writes remaining quota per tier and flags a failed query", async () => {
    const { subscriptionQuotaState } = await import(
      "@/components/settings/auth/AccountQuota"
    );
    const ok = subscriptionQuotaState(
      t,
      {
        tool: "codex",
        credentialStatus: "valid",
        credentialMessage: null,
        success: true,
        tiers: [
          { name: "five_hour", utilization: 38, resetsAt: null },
          { name: "seven_day", utilization: 95, resetsAt: null },
        ],
        extraUsage: null,
        error: null,
        queriedAt: 1,
      },
      false,
      "zh",
    );
    expect(ok?.kind).toBe("rows");
    if (ok?.kind === "rows") {
      expect(ok.rows.map((row) => [row.line.left, row.line.tone])).toEqual([
        [62, "normal"],
        [5, "warning"],
      ]);
    }

    expect(
      subscriptionQuotaState(
        t,
        {
          tool: "codex",
          credentialStatus: "valid",
          credentialMessage: null,
          success: false,
          tiers: [],
          extraUsage: null,
          error: "HTTP 500",
          queriedAt: 1,
        },
        false,
        "zh",
      ),
    ).toEqual({ kind: "failed", reason: "HTTP 500" });
    expect(subscriptionQuotaState(t, undefined, true, "zh")).toEqual({
      kind: "loading",
    });
  });
});

describe("AccountQuotaColumn", () => {
  it("opens the saved resets to list when each one expires", async () => {
    const user = userEvent.setup();
    const { AccountQuotaColumn } = await import(
      "@/components/settings/auth/AccountQuota"
    );
    render(
      <AccountQuotaColumn
        login="me@example.com"
        loading={false}
        onRefresh={vi.fn()}
        state={{
          kind: "rows",
          rows: [
            {
              label: "重置",
              line: {
                key: "reset_credits",
                text: "重置剩余 3 次",
                value: "剩余 3 次",
                caption: "10月6日到期",
                tone: "warning",
                left: Infinity,
                breakdown: {
                  title: "存下的限额重置",
                  openLabel: "查看 3 次重置各自的到期时间",
                  items: [
                    {
                      key: "a",
                      label: "10月6日",
                      hint: "2d0h后",
                      value: "2 次",
                      tone: "warning",
                    },
                    { key: "b", label: "不会过期", value: "1 次", tone: "normal" },
                  ],
                },
              },
            },
          ],
        }}
      />,
    );

    expect(screen.queryByText("存下的限额重置")).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "查看 3 次重置各自的到期时间" }),
    );
    const dialog = await screen.findByRole("dialog", { name: "存下的限额重置" });
    expect(
      within(dialog)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual(["10月6日2d0h后2 次", "不会过期1 次"]);
  });
});

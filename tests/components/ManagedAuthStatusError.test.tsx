import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";

const authMocks = vi.hoisted(() => ({
  refetchCodex: vi.fn(),
}));

const failedStatus = (refetchStatus: () => void) => ({
  accounts: [],
  isStatusSuccess: false,
  isStatusError: true,
  hasAnyAccount: false,
  pollingState: "idle" as const,
  deviceCode: null,
  error: null,
  isPolling: false,
  isAddingAccount: false,
  isRemovingAccount: false,
  addAccount: vi.fn(),
  removeAccount: vi.fn(),
  cancelAuth: vi.fn(),
  logout: vi.fn(),
  refetchStatus,
});

vi.mock("@/components/providers/forms/hooks/useCodexOauth", () => ({
  useCodexOauth: () => failedStatus(authMocks.refetchCodex),
}));

// 「N 个供应商在用」要读供应商列表（React Query）；这里不关心，给空
vi.mock("@/components/providers/forms/hooks/useManagedAccountUsers", () => ({
  useManagedAccountUsers: () => () => [],
}));

describe("managed auth status failures", () => {
  beforeEach(() => {
    authMocks.refetchCodex.mockResolvedValue(undefined);
  });

  it("shows a retryable error instead of an empty Codex account selector", () => {
    const onAccountSelect = vi.fn();
    render(
      <CodexOAuthSection
        mode="select"
        selectedAccountId="acct-existing"
        onAccountSelect={onAccountSelect}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "无法加载 ChatGPT 账号状态，请重试。",
    );
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(onAccountSelect).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(authMocks.refetchCodex).toHaveBeenCalledTimes(1);
  });
});

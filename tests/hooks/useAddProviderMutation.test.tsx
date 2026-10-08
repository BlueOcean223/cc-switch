import type { ReactNode } from "react";
import { act, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddProviderMutation } from "@/lib/query/mutations";
import type { Provider } from "@/types";

const apiMocks = vi.hoisted(() => ({
  add: vi.fn(),
  ensureGrokBuildOfficialProvider: vi.fn(),
  getAll: vi.fn(),
  updateTrayMenu: vi.fn(),
}));

const uuidMocks = vi.hoisted(() => ({
  generateUUID: vi.fn(),
}));

const toastMocks = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    add: (...args: unknown[]) => apiMocks.add(...args),
    ensureGrokBuildOfficialProvider: (...args: unknown[]) =>
      apiMocks.ensureGrokBuildOfficialProvider(...args),
    getAll: (...args: unknown[]) => apiMocks.getAll(...args),
    updateTrayMenu: (...args: unknown[]) => apiMocks.updateTrayMenu(...args),
  },
  sessionsApi: {},
  settingsApi: {},
}));

vi.mock("@/utils/uuid", () => ({
  generateUUID: () => uuidMocks.generateUUID(),
}));

vi.mock("sonner", () => ({
  toast: toastMocks,
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  return { wrapper };
}

beforeEach(() => {
  apiMocks.add.mockReset().mockResolvedValue(true);
  apiMocks.ensureGrokBuildOfficialProvider.mockReset().mockResolvedValue(true);
  apiMocks.getAll.mockReset().mockResolvedValue({});
  apiMocks.updateTrayMenu.mockReset().mockResolvedValue(true);
  uuidMocks.generateUUID.mockReset().mockReturnValue("generated-uuid");
  toastMocks.success.mockReset();
  toastMocks.error.mockReset();
  toastMocks.warning.mockReset();
});

describe("useAddProviderMutation", () => {
  it("duplicates Grok Build official providers with a fresh id", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("grokbuild"), {
      wrapper,
    });

    const duplicatedProvider = await act(async () =>
      result.current.mutateAsync({
        name: "Grok Build Official copy",
        settingsConfig: { config: "" },
        category: "official",
      }),
    );

    expect(apiMocks.ensureGrokBuildOfficialProvider).not.toHaveBeenCalled();
    expect(apiMocks.add).toHaveBeenCalledTimes(1);
    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "generated-uuid",
        name: "Grok Build Official copy",
        category: "official",
      }),
      "grokbuild",
      undefined,
      undefined,
    );
    expect(duplicatedProvider.id).toBe("generated-uuid");
    expect(duplicatedProvider.id).not.toBe("grokbuild-official");
  });

  it("returns the persisted seed row for the Grok Build official preset", async () => {
    const seedProvider: Provider = {
      id: "grokbuild-official",
      name: "Grok Build Official",
      settingsConfig: { config: "" },
      category: "official",
      createdAt: 123,
    };
    apiMocks.getAll.mockResolvedValueOnce({
      "grokbuild-official": seedProvider,
    });
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("grokbuild"), {
      wrapper,
    });

    const persistedProvider = await act(async () =>
      result.current.mutateAsync({
        name: "Renamed by form",
        settingsConfig: { config: "ignored" },
        category: "official",
        ensureGrokBuildOfficialSeed: true,
      }),
    );

    expect(apiMocks.ensureGrokBuildOfficialProvider).toHaveBeenCalledTimes(1);
    expect(apiMocks.getAll).toHaveBeenCalledWith("grokbuild");
    expect(apiMocks.add).not.toHaveBeenCalled();
    expect(persistedProvider).toEqual(seedProvider);
  });

  it("adds a managed Codex account as a separate official card", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("codex"), {
      wrapper,
    });

    const persistedProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: {
          providerType: "codex_oauth",
          authBinding: {
            source: "managed_account",
            authProvider: "codex_oauth",
            accountId: "acct-managed",
          },
        },
      }),
    );

    expect(apiMocks.getAll).not.toHaveBeenCalled();
    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "generated-uuid",
        category: "official",
        meta: {
          providerType: "codex_oauth",
          authBinding: {
            source: "managed_account",
            authProvider: "codex_oauth",
            accountId: "acct-managed",
          },
        },
      }),
      "codex",
      undefined,
      undefined,
    );
    expect(persistedProvider).toEqual(
      expect.objectContaining({
        id: "generated-uuid",
        meta: expect.objectContaining({
          authBinding: expect.objectContaining({
            accountId: "acct-managed",
          }),
        }),
      }),
    );
  });

  it("adds every unbound Codex Official as an independent provider", async () => {
    uuidMocks.generateUUID
      .mockReset()
      .mockReturnValueOnce("unbound-official-1")
      .mockReturnValueOnce("unbound-official-2");
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("codex"), {
      wrapper,
    });

    const firstProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official 1",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: { providerType: "codex_oauth" },
      }),
    );
    const secondProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official 2",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: { providerType: "codex_oauth" },
      }),
    );

    expect(apiMocks.getAll).not.toHaveBeenCalled();
    expect(apiMocks.add).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        id: "unbound-official-1",
        meta: { providerType: "codex_oauth" },
      }),
      "codex",
      undefined,
      undefined,
    );
    expect(apiMocks.add).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        id: "unbound-official-2",
        meta: { providerType: "codex_oauth" },
      }),
      "codex",
      undefined,
      undefined,
    );
    expect(firstProvider.id).toBe("unbound-official-1");
    expect(secondProvider.id).toBe("unbound-official-2");
  });

  it("adds a Pi provider without a separate default-model command", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("pi"), {
      wrapper,
    });

    const provider = await act(async () =>
      result.current.mutateAsync({
        name: "Pi Provider",
        providerKey: "pi-provider",
        settingsConfig: {
          api: "openai-responses",
          baseUrl: "https://example.com/v1",
          apiKey: "secret",
          models: [{ id: "model-a" }],
        },
      }),
    );

    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({ id: "pi-provider" }),
      "pi",
      undefined,
      undefined,
    );
    expect(provider.id).toBe("pi-provider");
  });

  it("reports a Pi provider add failure", async () => {
    apiMocks.add.mockRejectedValueOnce(new Error("provider add failed"));
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("pi"), {
      wrapper,
    });

    await act(async () => {
      await expect(
        result.current.mutateAsync({
          name: "Pi Provider",
          providerKey: "pi-provider",
          settingsConfig: { models: [{ id: "model-a" }] },
        }),
      ).rejects.toThrow("provider add failed");
    });

    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({ id: "pi-provider" }),
      "pi",
      undefined,
      undefined,
    );
    expect(toastMocks.error).toHaveBeenCalled();
    expect(toastMocks.warning).not.toHaveBeenCalled();
  });
});

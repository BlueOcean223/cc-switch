import { fireEvent, screen } from "@testing-library/react";
import { renderWithQueryClient as render } from "../utils/testQueryClient";
import userEvent from "@testing-library/user-event";
import { parse as parseToml } from "smol-toml";
import { describe, expect, it, vi } from "vitest";
import { GrokBuildProviderForm } from "@/components/providers/forms/GrokBuildProviderForm";
import { grokBuildProviderPresets } from "@/config/grokBuildProviderPresets";

vi.mock("@/components/JsonEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange: (value: string) => void;
  }) => (
    <textarea
      aria-label="raw-config"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

describe("GrokBuildProviderForm", () => {
  it("offers curated Grok Build presets and applies one", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={() => {}}
        onCancel={() => {}}
      />,
    );

    // 国产官方直连（cn_official）不在 Grok Build 预设列表里
    expect(screen.queryByRole("button", { name: /BytePlus/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Kimi/ })).toBeNull();

    await user.click(screen.getByRole("button", { name: /OpenRouter/ }));

    const baseUrlInput =
      container.querySelector<HTMLInputElement>("#codexBaseUrl");
    const nameInput =
      container.querySelector<HTMLInputElement>('input[name="name"]');
    expect(baseUrlInput?.value).toBe("https://openrouter.ai/api/v1");
    expect(nameInput?.value).toBe("OpenRouter");
  });

  it("points the get-API-key link at the preset's apiKeyUrl", async () => {
    const user = userEvent.setup();
    const { container } = render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={() => {}}
        onCancel={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: /CherryIN/ }));

    const websiteInput = container.querySelector<HTMLInputElement>(
      'input[name="websiteUrl"]',
    );
    expect(websiteInput?.value).toBe("https://open.cherryin.ai");
    expect(screen.getByRole("link", { name: /API Key/ })).toHaveAttribute(
      "href",
      "https://open.cherryin.ai/console/token",
    );
  });

  it("writes the preset's api_backend", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: /OpenRouter/ }));
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "secret-key" },
    });
    await user.click(screen.getByRole("button", { name: "Save" }));

    const preset = grokBuildProviderPresets.find(
      (candidate) => candidate.name === "OpenRouter",
    );
    const settings = JSON.parse(onSubmit.mock.calls[0][0].settingsConfig);
    const config = parseToml(settings.config) as any;
    expect(config.model[config.models.default].api_backend).toBe(
      preset?.apiBackend,
    );
  });

  it("submits a complete config.toml payload and leaves api_backend to Grok", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    const { container } = render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );

    const nameInput =
      container.querySelector<HTMLInputElement>('input[name="name"]');
    const baseUrlInput =
      container.querySelector<HTMLInputElement>("#codexBaseUrl");
    expect(nameInput).not.toBeNull();
    expect(baseUrlInput).not.toBeNull();

    fireEvent.change(nameInput!, { target: { value: "Example Relay" } });
    fireEvent.change(baseUrlInput!, {
      target: { value: "https://relay.example.com/v1" },
    });
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "secret-key" },
    });
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const submitted = onSubmit.mock.calls[0][0];
    expect(submitted.icon).toBe("");
    const settings = JSON.parse(submitted.settingsConfig);
    const config = parseToml(settings.config) as any;

    expect(config.models.default).toBe("grok-4.5");
    expect(config.model["grok-4.5"]).toEqual({
      model: "grok-4.5",
      base_url: "https://relay.example.com/v1",
      name: "Example Relay",
      api_key: "secret-key",
      context_window: 500000,
    });
  });

  it("shows only the fields Grok Build reads", () => {
    const { container } = render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={() => {}}
        onCancel={() => {}}
      />,
    );

    expect(container.querySelector("#grokbuild-profile")).toBeNull();
    // 没写 api_backend 时显示 Grok 实际使用的 Chat Completions
    expect(container.querySelector("#grokbuild-api-backend")).toHaveTextContent(
      "OpenAI Chat Completions",
    );
    expect(container.querySelector("#grokbuild-context-window")).toHaveValue(
      500000,
    );
    // 直连没有格式转换：不再有上游格式和高级选项
    expect(screen.queryByText("上游格式")).toBeNull();
    expect(screen.queryByText("高级选项")).toBeNull();
  });

  it("keeps the api_backend already in config.toml", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    const configToml = `[models]
default = "grok-4.5"

[model."grok-4.5"]
model = "grok-4.5"
base_url = "https://relay.example.com/v1"
name = "Chat Relay"
api_key = "secret-key"
api_backend = "chat_completions"
context_window = 500000
`;
    render(
      <GrokBuildProviderForm
        providerId="chat-relay"
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={() => {}}
        initialData={{
          name: "Chat Relay",
          category: "custom",
          settingsConfig: { config: configToml },
        }}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const submitted = onSubmit.mock.calls[0][0];
    const settings = JSON.parse(submitted.settingsConfig);
    const config = parseToml(settings.config) as any;
    const selected = config.model[config.models.default];
    expect(selected.api_backend).toBe("chat_completions");
    expect(selected.model).toBe("grok-4.5");
    expect(selected.base_url).toBe("https://relay.example.com/v1");
  });

  it("renders localized validation feedback for malformed TOML", async () => {
    const onSubmit = vi.fn();
    render(
      <GrokBuildProviderForm
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );

    fireEvent.change(screen.getByLabelText("raw-config"), {
      target: { value: "[models" },
    });

    expect(screen.getByText(/Invalid config\.toml:/)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("loads edit-mode values and does not resubmit stale custom endpoints", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    const config = `[models]
default = "existing-profile"

[model."existing-profile"]
model = "grok-upstream"
base_url = "https://existing.example.com/v1"
name = "Existing Relay"
api_key = "existing-key"
api_backend = "responses"
context_window = 250000
`;
    const { container } = render(
      <GrokBuildProviderForm
        providerId="existing-provider"
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={() => {}}
        initialData={{
          name: "Existing Relay",
          settingsConfig: { config },
          meta: {
            custom_endpoints: {
              "https://deleted.example.com/v1": {
                url: "https://deleted.example.com/v1",
                addedAt: 1,
              },
            },
          },
        }}
      />,
    );

    expect(container.querySelector("#grokbuild-profile")).toBeNull();
    expect(
      container.querySelector<HTMLInputElement>("#codexDefaultModel")?.value,
    ).toBe("grok-upstream");
    expect(
      container.querySelector<HTMLInputElement>("#codexBaseUrl")?.value,
    ).toBe("https://existing.example.com/v1");

    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit.mock.calls[0][0].meta.custom_endpoints).toBeUndefined();
  });

  // #6427 复用 Codex 表单时把 Codex 专属文案原样带进了 Grok Build 表单。
  // 按 appId 分流后，Grok 表单不得再出现 Codex 字样或不适用条款（模型映射）。
  it("uses Grok-specific copy for the model field", () => {
    const configToml = `[models]
default = "grok-4.5"

[model."grok-4.5"]
model = "grok-4.5"
base_url = "https://relay.example.com/v1"
name = "Chat Relay"
api_key = "secret-key"
api_backend = "chat_completions"
context_window = 500000
`;
    const { container } = render(
      <GrokBuildProviderForm
        providerId="chat-relay"
        submitLabel="Save"
        onSubmit={() => {}}
        onCancel={() => {}}
        initialData={{
          name: "Chat Relay",
          category: "custom",
          settingsConfig: { config: configToml },
        }}
      />,
    );

    const modelInput =
      container.querySelector<HTMLInputElement>("#codexDefaultModel");
    expect(modelInput?.placeholder).toBe("例如: grok-4.5");
    expect(screen.getByText(/Grok Build 默认请求的模型/)).toBeInTheDocument();
    expect(screen.queryByText(/Codex 默认请求的模型/)).toBeNull();
    // Grok 没有模型映射目录，不应出现"映射第一行"条款
    expect(screen.queryByText(/映射第一行/)).toBeNull();
  });
});

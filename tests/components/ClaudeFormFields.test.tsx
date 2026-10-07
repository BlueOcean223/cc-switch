import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps, PropsWithChildren } from "react";
import { useForm } from "react-hook-form";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ClaudeFormFields } from "@/components/providers/forms/ClaudeFormFields";
import { Form } from "@/components/ui/form";

const modelFetchApiMock = vi.hoisted(() => ({
  fetchModelsForConfig: vi.fn(),
  showFetchModelsError: vi.fn(),
}));

vi.mock("@/lib/api/model-fetch", () => ({
  fetchModelsForConfig: modelFetchApiMock.fetchModelsForConfig,
  showFetchModelsError: modelFetchApiMock.showFetchModelsError,
}));

type ClaudeFormFieldsProps = ComponentProps<typeof ClaudeFormFields>;

const FormShell = ({ children }: PropsWithChildren) => {
  const form = useForm();

  return <Form {...form}>{children}</Form>;
};

const renderForm = (overrides: Partial<ClaudeFormFieldsProps> = {}) => {
  const props: ClaudeFormFieldsProps = {
    shouldShowApiKey: true,
    apiKey: "sk-test",
    onApiKeyChange: vi.fn(),
    category: "third_party",
    shouldShowApiKeyLink: false,
    websiteUrl: "",
    templateValueEntries: [],
    templateValues: {},
    templatePresetName: "",
    onTemplateValueChange: vi.fn(),
    shouldShowSpeedTest: false,
    baseUrl: "https://relay.example.com",
    onBaseUrlChange: vi.fn(),
    isEndpointModalOpen: false,
    onEndpointModalToggle: vi.fn(),
    onCustomEndpointsChange: vi.fn(),
    autoSelect: false,
    onAutoSelectChange: vi.fn(),
    showEndpointTools: true,
    shouldShowModelSelector: true,
    claudeModel: "",
    defaultHaikuModel: "",
    defaultHaikuModelName: "",
    defaultSonnetModel: "claude-sonnet",
    defaultSonnetModelName: "Claude Sonnet",
    defaultOpusModel: "",
    defaultOpusModelName: "",
    defaultFableModel: "",
    defaultFableModelName: "",
    subagentModel: "",
    onModelChange: vi.fn(),
    speedTestEndpoints: [],
    apiKeyField: "ANTHROPIC_AUTH_TOKEN",
    onApiKeyFieldChange: vi.fn(),
    ...overrides,
  };

  return render(
    <FormShell>
      <ClaudeFormFields {...props} />
    </FormShell>,
  );
};

describe("ClaudeFormFields", () => {
  beforeEach(() => {
    modelFetchApiMock.fetchModelsForConfig.mockReset().mockResolvedValue([]);
    modelFetchApiMock.showFetchModelsError.mockReset();
  });

  it("不会在表单打开时自动获取模型列表", () => {
    renderForm();

    expect(modelFetchApiMock.fetchModelsForConfig).not.toHaveBeenCalled();
  });

  it("点击获取模型列表后按当前地址和 Key 请求", async () => {
    renderForm();

    fireEvent.click(
      screen.getByRole("button", {
        name: "providerForm.fetchModels",
      }),
    );

    await waitFor(() => {
      expect(modelFetchApiMock.fetchModelsForConfig).toHaveBeenCalledWith(
        "https://relay.example.com",
        "sk-test",
        false,
        undefined,
      );
    });
  });

  it("缺地址或 Key 时只提示，不发请求", () => {
    renderForm({ apiKey: "" });

    fireEvent.click(
      screen.getByRole("button", {
        name: "providerForm.fetchModels",
      }),
    );

    expect(modelFetchApiMock.showFetchModelsError).toHaveBeenCalled();
    expect(modelFetchApiMock.fetchModelsForConfig).not.toHaveBeenCalled();
  });

  it("一键设置会同时写入 Subagent 模型", () => {
    const onModelChange = vi.fn();
    renderForm({
      claudeModel: "shared-model[1M]",
      defaultSonnetModel: "",
      defaultSonnetModelName: "",
      onModelChange,
    });

    fireEvent.click(
      screen.getByRole("button", {
        name: "一键设置",
      }),
    );

    expect(onModelChange).toHaveBeenCalledWith(
      "CLAUDE_CODE_SUBAGENT_MODEL",
      "shared-model[1M]",
    );
  });
});

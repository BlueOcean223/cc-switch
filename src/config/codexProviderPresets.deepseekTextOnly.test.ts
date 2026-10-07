import { describe, expect, it } from "vitest";

import { codexProviderPresets } from "./codexProviderPresets";

// DeepSeek 官方端点已把 `deepseek-v4-flash` 指向识图的 V4.1 Flash，全局纯文本
// 名单因此不再收录该 id（#7283）。千帆托管的仍是纯文本的 V4 部署（千帆文档明写
// 附图 400），必须靠预设行的显式声明兜住，否则会 fail-open 成"可附图"。
const textOnlyDeepSeekHosts = ["Baidu Qianfan"];

describe("third-party DeepSeek V4 rows stay text-only", () => {
  it.each(textOnlyDeepSeekHosts)("%s declares text-only", (name) => {
    const preset = codexProviderPresets.find((p) => p.name === name);
    expect(preset, name).toBeDefined();
    const rows = (preset!.modelCatalog ?? []).filter((row) =>
      row.model.startsWith("deepseek-v4-"),
    );
    expect(rows.length, `${name} carries DeepSeek V4 rows`).toBeGreaterThan(0);
    for (const row of rows) {
      expect(row.inputModalities, `${name}/${row.model}`).toEqual(["text"]);
    }
  });

  it("distinguishes official V4.1 Flash vision from text-only V4 Pro", () => {
    const preset = codexProviderPresets.find((p) => p.name === "DeepSeek");
    expect(preset).toBeDefined();
    expect(
      preset!.modelCatalog?.find((row) => row.model === "deepseek-flash")
        ?.inputModalities,
    ).toEqual(["text", "image"]);
    expect(
      preset!.modelCatalog?.find((row) => row.model === "deepseek-v4-pro")
        ?.inputModalities,
    ).toEqual(["text"]);
  });
});

import { describe, expect, it, vi } from "vitest";
import type { TFunction } from "i18next";
import type { Provider } from "@/types";
import {
  buildAdditiveSections,
  buildSwitchSections,
  type ProviderSection,
} from "@/components/providers/presentation";

const t = ((key: string, options?: Record<string, unknown>) =>
  options && "count" in options
    ? `${key}:${String(options.count)}`
    : key) as unknown as TFunction;

const provider = (id: string, overrides: Partial<Provider> = {}): Provider => ({
  id,
  name: id,
  settingsConfig: {},
  ...overrides,
});

const official = provider("official", { category: "official" });
const relay = provider("relay");
const backup = provider("backup");

const item = (sections: ProviderSection[], id: string) => {
  for (const section of sections) {
    const found = section.items.find((entry) => entry.provider.id === id);
    if (found) return { section: section.key, ...found.presentation };
  }
  throw new Error(`no card for ${id}`);
};

const button = (sections: ProviderSection[], id: string, key: string) => {
  const found = item(sections, id).buttons.find((b) => b.key === key);
  if (!found) throw new Error(`no ${key} button on ${id}`);
  return found;
};

describe("buildSwitchSections", () => {
  it("marks the current provider in use and switches to the others", () => {
    const onSwitch = vi.fn();
    const sections = buildSwitchSections({
      app: "claude",
      t,
      providers: [official, relay, backup],
      currentId: "relay",
      onSwitch,
    });

    expect(sections.map((s) => s.key)).toEqual(["all"]);
    expect(item(sections, "relay")).toMatchObject({
      tone: "direct",
      status: { label: "providerCard.status.inUse", dot: "direct" },
      buttons: [],
      // 后端不让删正在使用的那家
      deleteDisabledReason: "providerCard.reason.inUseCannotDelete",
    });
    expect(item(sections, "backup").tone).toBeUndefined();
    expect(item(sections, "backup").deleteDisabledReason).toBeUndefined();
    button(sections, "backup", "switch").onClick();
    expect(onSwitch).toHaveBeenCalledWith(backup);
  });

  it("disables switching to cards that need the removed local routing", () => {
    const copilot = provider("copilot", {
      meta: { providerType: "github_copilot" },
    });
    const sections = buildSwitchSections({
      app: "claude",
      t,
      providers: [relay, copilot],
      currentId: "relay",
      onSwitch: vi.fn(),
    });
    expect(item(sections, "copilot").chips).toEqual([
      expect.objectContaining({
        key: "needsRouting",
        label: "providerCard.chip.needsRouting",
        tone: "warning",
      }),
    ]);
    expect(button(sections, "copilot", "switch").disabledReason).toBe(
      "providerCard.reason.needsRouting",
    );
    expect(item(sections, "relay").chips).toEqual([]);
  });

  it("offers a rewrite on the current card when live is left on upstream routing", () => {
    const onReapply = vi.fn();
    const build = (upstreamActive: boolean) =>
      buildSwitchSections({
        app: "claude",
        t,
        providers: [relay, backup],
        currentId: "relay",
        onSwitch: vi.fn(),
        liveRouting: { baseUrl: "http://127.0.0.1:15721", upstreamActive },
        onReapply,
      });

    const residue = item(build(false), "relay");
    expect(residue.status).toBeUndefined();
    expect(residue.chips).toEqual([
      expect.objectContaining({
        key: "liveRouting",
        label: "providerCard.chip.routingResidue",
        tone: "warning",
      }),
    ]);
    button(build(false), "relay", "reapply").onClick();
    expect(onReapply).toHaveBeenCalledTimes(1);

    expect(item(build(true), "relay").chips[0].label).toBe(
      "providerCard.chip.upstreamRouting",
    );
    // 其他卡不受影响
    expect(button(build(false), "backup", "switch")).toBeDefined();
  });

  it("tags official accounts, including legacy Codex cards without a category", () => {
    const legacyManaged = provider("legacy-managed", {
      settingsConfig: { auth: {}, config: null },
      meta: {
        authBinding: {
          source: "managed_account",
          authProvider: "codex_oauth",
          accountId: "acct-managed",
        },
      },
    });
    const sections = buildSwitchSections({
      app: "codex",
      t,
      providers: [official, relay, legacyManaged],
      currentId: "relay",
      onSwitch: vi.fn(),
    });

    expect(item(sections, "official").chips.map((c) => c.key)).toEqual([
      "official",
    ]);
    expect(item(sections, "legacy-managed").chips.map((c) => c.key)).toEqual([
      "official",
    ]);
    expect(item(sections, "relay").chips).toEqual([]);
  });
});

describe("buildAdditiveSections", () => {
  const additiveActions = () => ({
    add: vi.fn(),
    remove: vi.fn(),
    disableOmo: vi.fn(),
    setDefault: vi.fn(),
  });

  it("splits added and available providers and keeps OMO to one at a time", () => {
    const omo = provider("omo", { category: "omo" });
    const otherOmo = provider("omo-2", { category: "omo" });
    const actions = additiveActions();
    const sections = buildAdditiveSections({
      app: "opencode",
      t,
      providers: [relay, backup, omo, otherOmo],
      isInConfig: (p) => p.id === "relay",
      currentOmoId: "omo",
      actions,
    });

    expect(sections.map((s) => [s.key, s.items.length])).toEqual([
      ["added", 2],
      ["available", 2],
    ]);
    button(sections, "relay", "remove").onClick();
    expect(actions.remove).toHaveBeenCalledWith(relay);
    // 已添加的卡白底、不再写「● 已添加 / 已启用」：灰底只给默认那一家
    for (const id of ["relay", "omo"]) {
      expect(item(sections, id).tone).toBeUndefined();
      expect(item(sections, id).status).toBeUndefined();
    }
    button(sections, "backup", "add").onClick();
    expect(actions.add).toHaveBeenCalledWith(backup);
    button(sections, "omo", "disable").onClick();
    expect(actions.disableOmo).toHaveBeenCalledWith(omo);
    expect(item(sections, "omo-2").buttons.map((b) => b.key)).toEqual([
      "enable",
    ]);
  });

  it("lets OpenClaw pick the default model and protects the current default", () => {
    const multi = provider("multi", {
      settingsConfig: {
        models: [{ id: "m-1", name: "Model 1" }, { id: "m-2" }],
      },
    });
    const actions = additiveActions();
    const sections = buildAdditiveSections({
      app: "openclaw",
      t,
      providers: [relay, multi],
      isInConfig: () => true,
      openclawDefault: { providerId: "relay", model: "gpt" },
      openclawModels: (p) =>
        (p.settingsConfig as { models?: { id: string; name?: string }[] })
          .models ?? [],
      actions,
    });

    expect(item(sections, "relay")).toMatchObject({
      tone: "neutral",
      status: { label: "providerCard.status.default" },
    });
    expect(item(sections, "multi").tone).toBeUndefined();
    expect(item(sections, "multi").status).toBeUndefined();
    expect(button(sections, "relay", "remove").disabledReason).toBe(
      "providerCard.reason.defaultCannotRemove",
    );
    const setDefault = button(sections, "multi", "setDefault");
    expect(setDefault.menu?.options.map((o) => [o.label, o.detail])).toEqual([
      ["Model 1", "m-1"],
      ["m-2", undefined],
    ]);
    setDefault.menu?.options[1].onSelect();
    expect(actions.setDefault).toHaveBeenCalledWith(multi, "m-2");
  });

  it("keeps Hermes-managed providers read-only", () => {
    const managed = provider("managed");
    const sections = buildAdditiveSections({
      app: "hermes",
      t,
      providers: [relay, managed],
      isInConfig: () => true,
      hermesCurrentId: "relay",
      isHermesManaged: (p) => p.id === "managed",
      actions: additiveActions(),
    });

    expect(button(sections, "relay", "remove").disabledReason).toBe(
      "providerCard.reason.currentCannotRemove",
    );
    expect(item(sections, "relay").tone).toBe("neutral");
    expect(item(sections, "managed").tone).toBeUndefined();
    expect(item(sections, "managed")).toMatchObject({
      dim: true,
      editDisabledReason: "provider.managedByHermesHint",
      deleteDisabledReason: "provider.managedByHermesHint",
    });
    expect(button(sections, "managed", "use").disabledReason).toBe(
      "provider.managedByHermesHint",
    );
  });

  it("blocks every Pi change while Pi's current state is unavailable", () => {
    const sections = buildAdditiveSections({
      app: "pi",
      t,
      providers: [relay],
      isInConfig: () => false,
      piStateUnavailable: true,
      actions: additiveActions(),
    });

    expect(button(sections, "relay", "add")).toMatchObject({
      label: "providerCard.action.enable",
      disabledReason: "pi.current.stateUnavailableHint",
    });
    expect(item(sections, "relay").deleteDisabledReason).toBe(
      "pi.current.stateUnavailableHint",
    );
  });
});

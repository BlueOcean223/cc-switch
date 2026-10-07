import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProviderCardActions } from "@/components/providers/ProviderCardActions";
import type { CardPresentation } from "@/components/providers/presentation";

async function openMenu(
  presentation: CardPresentation,
  onDelete: () => void = vi.fn(),
) {
  const user = userEvent.setup();
  render(
    <ProviderCardActions
      providerName="Kimi"
      presentation={presentation}
      onEdit={vi.fn()}
      onDelete={onDelete}
      onDuplicate={vi.fn()}
    />,
  );
  await user.click(
    screen.getByRole("button", { name: "providerCard.action.more" }),
  );
  return { user, menu: await screen.findByRole("menu") };
}

describe("ProviderCardActions — the more menu", () => {
  it("shows the usual items", async () => {
    const { menu } = await openMenu({ buttons: [], chips: [] });

    expect(
      within(menu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent),
    ).toEqual(["provider.duplicate", "common.delete"]);
  });

  it("keeps delete listed with its reason when it is not allowed", async () => {
    const onDelete = vi.fn();
    const { user, menu } = await openMenu(
      {
        buttons: [],
        chips: [],
        deleteDisabledReason: "providerCard.reason.inUseCannotDelete",
      },
      onDelete,
    );

    const item = within(menu).getAllByRole("menuitem").at(-1)!;
    expect(item).toHaveAttribute("aria-disabled", "true");
    expect(item).toHaveTextContent("providerCard.reason.inUseCannotDelete");
    await user.click(item);
    expect(onDelete).not.toHaveBeenCalled();
  });
});

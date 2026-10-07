import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApiKeySection } from "@/components/providers/forms/shared/ApiKeySection";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

function renderSection(
  props: Partial<React.ComponentProps<typeof ApiKeySection>> = {},
) {
  return render(
    <ApiKeySection
      value=""
      onChange={() => {}}
      category="third_party"
      shouldShowLink
      websiteUrl="https://example.com/keys"
      {...props}
    />,
  );
}

describe("ApiKeySection", () => {
  it("shows the API Key link without a hint under the input", () => {
    const { container } = renderSection();

    expect(
      screen.getByRole("link", { name: /providerForm\.getApiKey/ }),
    ).toHaveAttribute("href", "https://example.com/keys");
    expect(container.querySelector("p")).toBeNull();
  });

  it("hides the API Key link when shouldShowLink is false", () => {
    renderSection({ category: "official", shouldShowLink: false });

    expect(
      screen.queryByRole("link", { name: /providerForm\.getApiKey/ }),
    ).not.toBeInTheDocument();
  });
});

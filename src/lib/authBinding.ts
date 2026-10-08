import type { ProviderMeta } from "@/types";

export function resolveManagedAccountId(
  meta: ProviderMeta | undefined,
  authProvider: string,
): string | null {
  const binding = meta?.authBinding;

  if (
    binding?.source === "managed_account" &&
    binding.authProvider === authProvider
  ) {
    return binding.accountId ?? null;
  }

  return null;
}

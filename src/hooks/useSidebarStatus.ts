import { useQuery } from "@tanstack/react-query";
import * as authApi from "@/lib/api/auth";
import { useUsageSummary } from "@/lib/query/usage";
import { parseFiniteNumber } from "@/components/usage/format";
import type { UsageRangeSelection } from "@/types/usage";

const TODAY: UsageRangeSelection = { preset: "today" };

/**
 * 侧栏需要的状态：今日花费、授权中心是否有账号要重新登录。
 */
export function useSidebarStatus() {
  const { data: todaySummary } = useUsageSummary(TODAY, undefined, {
    refetchInterval: 60_000,
  });

  const { data: codexAuthStatus } = useQuery({
    queryKey: ["managed-auth-status", "codex_oauth"],
    queryFn: () => authApi.authGetStatus(),
    staleTime: 60_000,
  });

  const todayCost = parseFiniteNumber(todaySummary?.totalCost) ?? 0;

  const authNeedsAttention =
    codexAuthStatus?.accounts.some(
      (account) => account.reauth_required === true,
    ) ?? false;

  return { todayCost, authNeedsAttention };
}

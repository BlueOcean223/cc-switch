import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Globe, Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { HoverTip } from "@/components/ui/hover-tip";
import { HelpTip } from "@/components/ui/help-tip";
import { Notice } from "@/components/ui/notice";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { useModelPricing, useDeleteModelPricing } from "@/lib/query/usage";
import { TablePagination, useClientPagination } from "./TablePagination";
import type { ModelPricing } from "@/types/usage";
import { cn } from "@/lib/utils";
import { PricingEditModal } from "./PricingEditModal";
import { ModelsDevAutoSyncPanel } from "./ModelsDevAutoSyncPanel";
import { ModelsDevPickerDialog } from "./ModelsDevPickerDialog";
import { parseFiniteNumber } from "./format";
import { usageTable } from "./usageTable";

/** 「$4.00」「$0.075」：至少两位小数，多余的 0 去掉，最多 4 位。 */
export function formatUnitPrice(value: string): string {
  const num = parseFiniteNumber(value);
  if (num == null) return `$${value}`;
  const fixed = num.toFixed(4).replace(/0+$/, "");
  const [whole, frac = ""] = fixed.split(".");
  return `$${whole}.${frac.padEnd(2, "0")}`;
}

/** 定价子页签（v7 S6）：models.dev 同步、模型定价表。 */
export function PricingConfigPanel() {
  const { t } = useTranslation();
  const { data: pricing, isLoading, error } = useModelPricing();
  const deleteMutation = useDeleteModelPricing();
  const [editingModel, setEditingModel] = useState<ModelPricing | null>(null);
  const [isAddingNew, setIsAddingNew] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ModelPricing | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);

  const handleAddNew = () => {
    setIsAddingNew(true);
    setEditingModel({
      modelId: "",
      displayName: "",
      inputCostPerMillion: "0",
      outputCostPerMillion: "0",
      cacheReadCostPerMillion: "0",
      cacheCreationCostPerMillion: "0",
    });
  };

  const rows = pricing ?? [];
  const pagination = useClientPagination(rows);

  return (
    <div className="flex flex-col gap-3 pt-3">
      <ModelsDevAutoSyncPanel />

      <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-2">
        <div className="flex items-center gap-0.5">
          <h3 className="m-0 whitespace-nowrap text-strong font-semibold text-fg-1">
            {t("usage.pricing.tableTitle", { count: rows.length })}
          </h3>
          <HelpTip title={t("usage.pricing.costHelpTitle")}>
            {t("usage.pricing.costHelp")}
          </HelpTip>
        </div>
        <span className="whitespace-nowrap text-caption text-fg-3">
          {t("usage.pricing.unit")}
        </span>
        <div className="flex-1" />
        <Button
          type="button"
          variant="neutral"
          size="compact"
          onClick={() => setPickerOpen(true)}
        >
          <Globe className="h-3.5 w-3.5" />
          {t("usage.importFromModelsDev")}
        </Button>
        <Button
          type="button"
          variant="neutral"
          size="compact"
          onClick={handleAddNew}
        >
          <Plus className="h-3.5 w-3.5" />
          {t("usage.addPricing")}
        </Button>
      </div>

      {isLoading ? (
        <div className={usageTable.skeleton} />
      ) : error ? (
        <Notice
          tone="danger"
          title={`${t("usage.loadPricingError")}: ${String(error)}`}
        />
      ) : (
        <div className={usageTable.scroller}>
          <table
            className={cn(usageTable.table, "min-w-[620px]")}
            aria-label={t("usage.modelPricing")}
          >
            <thead>
              <tr className={usageTable.headRow}>
                <th className={usageTable.th}>{t("usage.model")}</th>
                <th className={usageTable.thEnd}>{t("usage.inputTokens")}</th>
                <th className={usageTable.thEnd}>{t("usage.outputTokens")}</th>
                <th className={usageTable.thEnd}>
                  {t("usage.cacheReadTokens")}
                </th>
                <th className={usageTable.thEnd}>
                  {t("usage.pricing.cacheWrite")}
                </th>
                <th className="w-[72px]" aria-label={t("common.actions")} />
              </tr>
            </thead>
            <tbody>
              {rows.length === 0 ? (
                <tr>
                  <td colSpan={6} className={usageTable.empty}>
                    {t("usage.noPricingData")}
                  </td>
                </tr>
              ) : (
                pagination.pageRows.map((model) => {
                  const name = model.displayName || model.modelId;
                  return (
                    <tr
                      key={model.modelId}
                      className={cn(usageTable.row, "group hover:bg-subtle")}
                    >
                      <td className={usageTable.td}>
                        <span className="flex max-w-[340px] items-baseline gap-2">
                          {model.displayName && (
                            <span className="truncate text-fg-1">
                              {model.displayName}
                            </span>
                          )}
                          <span
                            className="truncate font-mono text-caption text-fg-2"
                            title={model.modelId}
                          >
                            {model.modelId}
                          </span>
                        </span>
                      </td>
                      <td className={usageTable.tdEnd}>
                        {formatUnitPrice(model.inputCostPerMillion)}
                      </td>
                      <td className={usageTable.tdEnd}>
                        {formatUnitPrice(model.outputCostPerMillion)}
                      </td>
                      <td className={usageTable.tdEnd}>
                        {formatUnitPrice(model.cacheReadCostPerMillion)}
                      </td>
                      <td className={cn(usageTable.tdEnd, "text-fg-2")}>
                        {formatUnitPrice(model.cacheCreationCostPerMillion)}
                      </td>
                      <td className="whitespace-nowrap pe-1 text-end">
                        <HoverTip content={t("common.edit")}>
                          <Button
                            type="button"
                            variant="quiet"
                            size="icon-compact"
                            aria-label={t("usage.pricing.editAria", { name })}
                            onClick={() => {
                              setIsAddingNew(false);
                              setEditingModel(model);
                            }}
                          >
                            <Pencil
                              className="h-3.5 w-3.5"
                              strokeWidth={1.75}
                            />
                          </Button>
                        </HoverTip>
                        <HoverTip content={t("common.delete")}>
                          <Button
                            type="button"
                            variant="quiet"
                            size="icon-compact"
                            aria-label={t("usage.pricing.deleteAria", { name })}
                            className="hover:text-danger-text"
                            onClick={() => setDeleteTarget(model)}
                          >
                            <Trash2
                              className="h-3.5 w-3.5"
                              strokeWidth={1.75}
                            />
                          </Button>
                        </HoverTip>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      )}
      {!isLoading && !error && (
        <TablePagination
          page={pagination.page}
          totalPages={pagination.totalPages}
          total={pagination.total}
          onPageChange={pagination.setPage}
        />
      )}

      {editingModel && (
        <PricingEditModal
          open={!!editingModel}
          model={editingModel}
          isNew={isAddingNew}
          onClose={() => {
            setEditingModel(null);
            setIsAddingNew(false);
          }}
        />
      )}

      {pickerOpen && (
        <ModelsDevPickerDialog
          open={pickerOpen}
          onClose={() => setPickerOpen(false)}
          onImported={() => setPickerOpen(false)}
        />
      )}

      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t("usage.deleteConfirmTitle")}
        message={t("usage.deleteConfirmDesc")}
        confirmText={
          deleteMutation.isPending ? t("common.deleting") : t("common.delete")
        }
        variant="destructive"
        onConfirm={() => {
          if (!deleteTarget) return;
          deleteMutation.mutate(deleteTarget.modelId, {
            onSuccess: () => setDeleteTarget(null),
          });
        }}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}

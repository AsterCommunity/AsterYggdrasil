import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { RefreshButton } from "@/components/common/RefreshButton";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { TooltipProvider } from "@/components/ui/tooltip";
import { parsePageSizeOption } from "@/lib/pagination";
import { ForwardServerDialog } from "./ForwardServerDialog";
import {
	ForwardServerRow,
	ForwardServerTableHeader,
} from "./ForwardServerTable";
import { PAGE_SIZE_OPTIONS, parseForwardSort } from "./forwardServerForm";
import {
	type AdminYggdrasilForwardingPageController,
	useAdminYggdrasilForwardingPageController,
} from "./useAdminYggdrasilForwardingPageController";

export default function AdminYggdrasilForwardingPage() {
	const controller = useAdminYggdrasilForwardingPageController();
	return <AdminYggdrasilForwardingPageLayout controller={controller} />;
}

function AdminYggdrasilForwardingPageLayout({
	controller,
}: {
	controller: AdminYggdrasilForwardingPageController;
}) {
	const { t } = useTranslation();
	const {
		currentPage,
		deletingServer,
		deleteDialogOpen,
		dialogOpen,
		editingServer,
		form,
		list,
		onDeleteDialogOpenChange,
		onDeleteServer,
		onDialogOpenChange,
		onOpenCreate,
		onOpenDelete,
		onOpenEdit,
		onPatchServer,
		onSetField,
		onSubmitForm,
		pageSize,
		pageSizeOptions,
		selectedSortLabel,
		sort,
		sortOptions,
		submitting,
		totalPages,
		updatingId,
		updatePagination,
	} = controller;

	const headerRow = useMemo(() => <ForwardServerTableHeader />, []);
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={list.total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={list.prevPageDisabled}
				nextDisabled={list.nextPageDisabled}
				onPrevious={list.goPrevious}
				onNext={list.goNext}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, PAGE_SIZE_OPTIONS);
					if (next == null) return;
					updatePagination({ pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			list.goNext,
			list.goPrevious,
			list.nextPageDisabled,
			list.prevPageDisabled,
			list.total,
			pageSize,
			pageSizeOptions,
			totalPages,
			updatePagination,
		],
	);

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.yggdrasilForwarding.title")}
				description={t("admin.yggdrasilForwarding.description")}
				actions={
					<>
						<Select
							value={sort}
							onValueChange={(value) =>
								updatePagination({
									sort: parseForwardSort(value),
								})
							}
						>
							<SelectTrigger
								width="fit"
								aria-label={t("admin.yggdrasilForwarding.sort.label")}
							>
								<Icon name="Shuffle" className="size-4 text-muted-foreground" />
								<SelectValue>{selectedSortLabel}</SelectValue>
							</SelectTrigger>
							<SelectContent align="end">
								{sortOptions.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										{option.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<Button type="button" size="sm" onClick={onOpenCreate}>
							<Icon name="Plus" className="mr-2 size-4" />
							{t("admin.yggdrasilForwarding.createAction")}
						</Button>
						<RefreshButton
							loading={list.loading}
							onClick={() => void list.reload()}
						/>
					</>
				}
			/>

			<TooltipProvider delay={100}>
				<AdminTableList
					loading={list.loading}
					items={list.items}
					columns={5}
					rows={6}
					emptyIcon={<Icon name="Shuffle" className="size-5" />}
					emptyTitle={t("admin.yggdrasilForwarding.emptyTitle")}
					emptyDescription={t("admin.yggdrasilForwarding.emptyDescription")}
					emptyAction={
						<Button type="button" size="sm" onClick={onOpenCreate}>
							<Icon name="Plus" className="mr-2 size-4" />
							{t("admin.yggdrasilForwarding.createAction")}
						</Button>
					}
					headerRow={headerRow}
					pagination={pagination}
					renderRow={(server) => (
						<ForwardServerRow
							key={server.id}
							server={server}
							updating={updatingId === server.id}
							onEdit={onOpenEdit}
							onDelete={onOpenDelete}
							onPatch={(patch) => onPatchServer(server, patch)}
						/>
					)}
				/>
			</TooltipProvider>

			<ForwardServerDialog
				form={form}
				open={dialogOpen}
				server={editingServer}
				submitting={submitting}
				onOpenChange={onDialogOpenChange}
				onSubmit={onSubmitForm}
				onFieldChange={onSetField}
			/>

			<ConfirmDialog
				open={deleteDialogOpen}
				loading={updatingId === deletingServer?.id}
				title={t("admin.yggdrasilForwarding.deleteTitle")}
				description={t("admin.yggdrasilForwarding.deleteDescription", {
					name: deletingServer?.display_name ?? "",
				})}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				variant="destructive"
				onOpenChange={onDeleteDialogOpenChange}
				onConfirm={onDeleteServer}
			/>
		</AdminPageShell>
	);
}

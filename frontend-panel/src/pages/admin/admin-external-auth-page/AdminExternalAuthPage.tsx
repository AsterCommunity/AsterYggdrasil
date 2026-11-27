import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { RefreshButton } from "@/components/common/RefreshButton";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { DialogLifecycle } from "@/components/ui/dialog-lifecycle";
import { Icon } from "@/components/ui/icon";
import { parsePageSizeOption } from "@/lib/pagination";
import type { AdminExternalAuthProviderInfo } from "@/types/api";
import { ExternalAuthProviderDialog } from "./ExternalAuthProviderDialog";
import {
	ExternalAuthProvidersTableHeader,
	ExternalAuthProvidersTableRow,
} from "./ExternalAuthProvidersTable";
import { callbackUrl, EXTERNAL_AUTH_PAGE_SIZE_OPTIONS } from "./shared";
import {
	type AdminExternalAuthPageController,
	type ExternalAuthUiState,
	useAdminExternalAuthPageController,
} from "./useAdminExternalAuthPageController";

export default function AdminExternalAuthPage() {
	const controller = useAdminExternalAuthPageController();
	return <AdminExternalAuthPageLayout controller={controller} />;
}

function AdminExternalAuthPageLayout({
	controller,
}: {
	controller: AdminExternalAuthPageController;
}) {
	const { t } = useTranslation();
	const {
		actions,
		currentPage,
		list,
		openCreate,
		openEdit,
		pageSize,
		pageSizeOptions,
		state,
		totalPages,
		updatePagination,
	} = controller;

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.externalAuth.title")}
				description={t("admin.externalAuth.description")}
				actions={
					<ExternalAuthPageActions
						loading={list.loading}
						onCreate={openCreate}
						onRefresh={() => void list.reload()}
					/>
				}
			/>

			<ExternalAuthProviderList
				currentPage={currentPage}
				hasNextPage={list.nextCursor !== null}
				pageSize={pageSize}
				pageSizeOptions={pageSizeOptions}
				state={state}
				totalPages={totalPages}
				providers={list.items}
				loading={list.loading}
				total={list.total}
				onCopyCallbackUrl={actions.copyCallback}
				onCreate={openCreate}
				onEdit={openEdit}
				onRequestDelete={(provider) =>
					controller.dispatch({
						type: "set_deleting_provider",
						value: provider,
					})
				}
				onTestProvider={(provider) => void actions.testProvider(provider)}
				onNextPage={list.goNext}
				onPreviousPage={list.goPrevious}
				onUpdatePagination={updatePagination}
			/>

			<ExternalAuthPageDialogs
				controller={controller}
				onCloseCreatedProvider={() =>
					controller.dispatch({ type: "set_created_provider", value: null })
				}
				onCloseDeleteProvider={() =>
					controller.dispatch({ type: "set_deleting_provider", value: null })
				}
				onCloseProviderDialog={() =>
					controller.dispatch({ type: "close_dialog" })
				}
				onDeleteProvider={() => void actions.deleteProvider()}
				onSubmit={() => void actions.submitProvider()}
				onTestConnection={() => void actions.testDraft()}
			/>
		</AdminPageShell>
	);
}

function ExternalAuthPageActions({
	loading,
	onCreate,
	onRefresh,
}: {
	loading: boolean;
	onCreate: () => void;
	onRefresh: () => void;
}) {
	const { t } = useTranslation();

	return (
		<>
			<Button type="button" size="sm" onClick={onCreate}>
				<Icon name="Plus" className="mr-2 size-4" />
				{t("admin.externalAuth.create")}
			</Button>
			<RefreshButton loading={loading} onClick={onRefresh} />
		</>
	);
}

function ExternalAuthProviderList({
	currentPage,
	hasNextPage,
	onCopyCallbackUrl,
	onCreate,
	onEdit,
	onNextPage,
	onPreviousPage,
	onRequestDelete,
	onTestProvider,
	onUpdatePagination,
	pageSize,
	pageSizeOptions,
	providers,
	loading,
	state,
	total,
	totalPages,
}: {
	currentPage: number;
	hasNextPage: boolean;
	onCopyCallbackUrl: (value: string) => void;
	onCreate: () => void;
	onEdit: (provider: AdminExternalAuthProviderInfo) => void;
	onNextPage: () => void;
	onPreviousPage: () => void;
	onRequestDelete: (provider: AdminExternalAuthProviderInfo) => void;
	onTestProvider: (provider: AdminExternalAuthProviderInfo) => void;
	onUpdatePagination: (value: {
		pageSize?: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
	}) => void;
	pageSize: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
	pageSizeOptions: { label: string; value: string }[];
	providers: AdminExternalAuthProviderInfo[];
	loading: boolean;
	state: ExternalAuthUiState;
	total: number;
	totalPages: number;
}) {
	const { t } = useTranslation();
	const emptyIcon = useMemo(() => <Icon name="Globe" className="size-5" />, []);
	const emptyAction = useMemo(
		() => (
			<Button type="button" size="sm" onClick={onCreate}>
				<Icon name="Plus" className="mr-2 size-4" />
				{t("admin.externalAuth.create")}
			</Button>
		),
		[onCreate, t],
	);
	const headerRow = useMemo(() => <ExternalAuthProvidersTableHeader />, []);
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={currentPage <= 1}
				nextDisabled={!hasNextPage}
				onPrevious={onPreviousPage}
				onNext={onNextPage}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(
						value,
						EXTERNAL_AUTH_PAGE_SIZE_OPTIONS,
					);
					if (next == null) return;
					onUpdatePagination({ pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			hasNextPage,
			onNextPage,
			onPreviousPage,
			onUpdatePagination,
			pageSize,
			pageSizeOptions,
			total,
			totalPages,
		],
	);

	return (
		<AdminTableList
			loading={loading}
			items={providers}
			columns={4}
			rows={6}
			emptyIcon={emptyIcon}
			emptyTitle={t("admin.externalAuth.emptyTitle")}
			emptyDescription={t("admin.externalAuth.emptyDescription")}
			emptyAction={emptyAction}
			headerRow={headerRow}
			pagination={pagination}
			renderRow={(provider) => (
				<ExternalAuthProvidersTableRow
					key={provider.id}
					provider={provider}
					providerKinds={state.providerKinds}
					deletingId={state.deletingProvider?.id ?? null}
					testingId={state.testingId}
					onEdit={onEdit}
					onCopyCallbackUrl={onCopyCallbackUrl}
					onTestProvider={onTestProvider}
					onRequestDelete={onRequestDelete}
				/>
			)}
		/>
	);
}

function ExternalAuthPageDialogs({
	controller,
	onCloseCreatedProvider,
	onCloseDeleteProvider,
	onCloseProviderDialog,
	onDeleteProvider,
	onSubmit,
	onTestConnection,
}: {
	controller: AdminExternalAuthPageController;
	onCloseCreatedProvider: () => void;
	onCloseDeleteProvider: () => void;
	onCloseProviderDialog: () => void;
	onDeleteProvider: () => void;
	onSubmit: () => void;
	onTestConnection: () => void;
}) {
	const { t } = useTranslation();
	const { actions, createSteps, state, stepAnimationRef } = controller;

	// The dialog-lifecycle session retains the provider dialog's content
	// (form draft included) through the exit animation; the page reducer is
	// free to reset its state as soon as the close is requested.
	const dialogTarget = {
		createStep: state.createStep,
		createStepDirection: stepAnimationRef.current,
		createStepTouched: state.createStepTouched,
		createSteps,
		form: state.form,
		mode: state.editingProvider ? ("edit" as const) : ("create" as const),
		provider: state.editingProvider,
		providerKinds: state.providerKinds,
		submitting: state.submitting,
		testResult: state.testResult,
		testing: state.testingDraft,
	};

	return (
		<>
			<DialogLifecycle
				open={state.dialogOpen}
				target={dialogTarget}
				onOpenChange={(open) => {
					if (!open) onCloseProviderDialog();
				}}
			>
				{({ target, active, onOpenChange }) => (
					<ExternalAuthProviderDialog
						createStep={target.createStep}
						createStepDirection={target.createStepDirection}
						createStepTouched={target.createStepTouched}
						createSteps={target.createSteps}
						form={target.form}
						mode={target.mode}
						open={active}
						provider={target.provider}
						providerKinds={target.providerKinds}
						submitting={target.submitting}
						testResult={target.testResult}
						testing={target.testing}
						onCopyCallbackUrl={actions.copyCallback}
						onCreateBack={() =>
							controller.dispatch({
								type: "set_create_step",
								value: state.createStep - 1,
							})
						}
						onCreateNext={actions.goCreateNext}
						onCreateStepChange={(value) =>
							controller.dispatch({ type: "set_create_step", value })
						}
						onFieldChange={actions.setField}
						onOpenChange={onOpenChange}
						onProviderKindChange={actions.setProviderKind}
						onSubmit={onSubmit}
						onTestConnection={onTestConnection}
					/>
				)}
			</DialogLifecycle>

			<ConfirmDialog
				open={Boolean(state.deletingProvider)}
				onOpenChange={(open) => {
					if (!open) onCloseDeleteProvider();
				}}
				title={t("admin.externalAuth.deleteTitle", {
					name: state.deletingProvider?.display_name ?? "",
				})}
				description={t("admin.externalAuth.deleteDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				variant="destructive"
				onConfirm={onDeleteProvider}
			/>

			<ConfirmDialog
				open={Boolean(state.createdProvider)}
				onOpenChange={(open) => {
					if (!open) onCloseCreatedProvider();
				}}
				title={t("admin.externalAuth.callbackTitle")}
				description={
					state.createdProvider ? callbackUrl(state.createdProvider) : undefined
				}
				cancelLabel={t("common.close")}
				confirmLabel={t("admin.externalAuth.copyCallback")}
				onConfirm={() => {
					if (state.createdProvider) {
						void actions.copyCallback(callbackUrl(state.createdProvider));
					}
				}}
			/>
		</>
	);
}

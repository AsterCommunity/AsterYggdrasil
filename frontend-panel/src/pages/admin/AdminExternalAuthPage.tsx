import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { ExternalAuthProviderDialog } from "@/components/admin/admin-external-auth-page/ExternalAuthProviderDialog";
import {
	ExternalAuthProvidersTableHeader,
	ExternalAuthProvidersTableRow,
} from "@/components/admin/admin-external-auth-page/ExternalAuthProvidersTable";
import {
	callbackUrl,
	createPayload,
	DEFAULT_EXTERNAL_AUTH_PAGE_SIZE,
	defaultScopesForKind,
	EXTERNAL_AUTH_PAGE_SIZE_OPTIONS,
	type ExternalAuthCreateStep,
	type ExternalAuthProviderFormData,
	emptyExternalAuthForm,
	formatTestResult,
	formFromProvider,
	requiredFieldsMissing,
	sortExternalAuthProviderKinds,
	testParamsPayload,
	updatePayload,
} from "@/components/admin/admin-external-auth-page/shared";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	parseOffsetSearchParam,
	parsePageSizeOption,
	parsePageSizeSearchParam,
} from "@/lib/pagination";
import { cn } from "@/lib/utils";
import { adminExternalAuthService } from "@/services/adminService";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthKind,
	ExternalAuthProviderKindInfo,
} from "@/types/api";

type UiState = {
	createStep: number;
	createStepTouched: boolean;
	createdProvider: AdminExternalAuthProviderInfo | null;
	deletingProvider: AdminExternalAuthProviderInfo | null;
	dialogOpen: boolean;
	editingProvider: AdminExternalAuthProviderInfo | null;
	form: ExternalAuthProviderFormData;
	loading: boolean;
	providerKinds: ExternalAuthProviderKindInfo[];
	providers: AdminExternalAuthProviderInfo[];
	submitting: boolean;
	testResult: string | null;
	testingDraft: boolean;
	testingId: number | null;
	total: number;
};

type UiAction =
	| { type: "set_loading"; value: boolean }
	| {
			type: "loaded";
			providerKinds: ExternalAuthProviderKindInfo[];
			providers: AdminExternalAuthProviderInfo[];
			total: number;
	  }
	| { type: "open_create"; form: ExternalAuthProviderFormData }
	| {
			type: "open_edit";
			form: ExternalAuthProviderFormData;
			provider: AdminExternalAuthProviderInfo;
	  }
	| { type: "close_dialog" }
	| {
			type: "set_form_field";
			key: keyof ExternalAuthProviderFormData;
			value: ExternalAuthProviderFormData[keyof ExternalAuthProviderFormData];
	  }
	| {
			type: "set_provider_kind";
			kind: ExternalAuthKind;
			scopes: string;
	  }
	| { type: "set_create_step"; value: number }
	| { type: "set_create_step_touched"; value: boolean }
	| { type: "set_submitting"; value: boolean }
	| { type: "set_testing_draft"; value: boolean }
	| { type: "set_testing_id"; value: number | null }
	| { type: "set_test_result"; value: string | null }
	| {
			type: "set_deleting_provider";
			value: AdminExternalAuthProviderInfo | null;
	  }
	| {
			type: "set_created_provider";
			value: AdminExternalAuthProviderInfo | null;
	  }
	| { type: "provider_updated"; value: AdminExternalAuthProviderInfo };

function initialState(): UiState {
	return {
		createStep: 0,
		createStepTouched: false,
		createdProvider: null,
		deletingProvider: null,
		dialogOpen: false,
		editingProvider: null,
		form: emptyExternalAuthForm,
		loading: true,
		providerKinds: [],
		providers: [],
		submitting: false,
		testResult: null,
		testingDraft: false,
		testingId: null,
		total: 0,
	};
}

function reducer(state: UiState, action: UiAction): UiState {
	switch (action.type) {
		case "set_loading":
			return { ...state, loading: action.value };
		case "loaded":
			return {
				...state,
				providerKinds: sortExternalAuthProviderKinds(action.providerKinds),
				providers: action.providers,
				total: action.total,
			};
		case "open_create":
			return {
				...state,
				createStep: 0,
				createStepTouched: false,
				dialogOpen: true,
				editingProvider: null,
				form: action.form,
				testResult: null,
			};
		case "open_edit":
			return {
				...state,
				createStep: 0,
				createStepTouched: false,
				dialogOpen: true,
				editingProvider: action.provider,
				form: action.form,
				testResult: null,
			};
		case "close_dialog":
			return {
				...state,
				createStep: 0,
				createStepTouched: false,
				dialogOpen: false,
				editingProvider: null,
				form: emptyExternalAuthForm,
				submitting: false,
				testResult: null,
			};
		case "set_form_field":
			return {
				...state,
				form: { ...state.form, [action.key]: action.value },
				testResult: null,
			};
		case "set_provider_kind":
			return {
				...state,
				form: {
					...state.form,
					authorizationUrl: "",
					issuerUrl: "",
					providerKind: action.kind,
					scopes: action.scopes,
					tokenUrl: "",
					userinfoUrl: "",
				},
				testResult: null,
			};
		case "set_create_step":
			return {
				...state,
				createStep: Math.max(0, Math.min(2, action.value)),
				createStepTouched: false,
			};
		case "set_create_step_touched":
			return { ...state, createStepTouched: action.value };
		case "set_submitting":
			return { ...state, submitting: action.value };
		case "set_testing_draft":
			return { ...state, testingDraft: action.value };
		case "set_testing_id":
			return { ...state, testingId: action.value };
		case "set_test_result":
			return { ...state, testResult: action.value };
		case "set_deleting_provider":
			return { ...state, deletingProvider: action.value };
		case "set_created_provider":
			return { ...state, createdProvider: action.value };
		case "provider_updated":
			return {
				...state,
				providers: state.providers.map((provider) =>
					provider.id === action.value.id ? action.value : provider,
				),
			};
	}
}

export default function AdminExternalAuthPage() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.externalAuth.title"));

	const offset = parseOffsetSearchParam(searchParams.get("offset"));
	const pageSize = parsePageSizeSearchParam(
		searchParams.get("pageSize"),
		EXTERNAL_AUTH_PAGE_SIZE_OPTIONS,
		DEFAULT_EXTERNAL_AUTH_PAGE_SIZE,
	);
	const [state, dispatch] = useReducer(reducer, undefined, initialState);
	const previousCreateStepRef = useRef(0);
	const stepAnimationRef = useRef<"idle" | "forward" | "backward">("idle");
	const selectedKind =
		state.providerKinds.find((kind) => kind.kind === state.form.providerKind) ??
		state.providerKinds[0] ??
		null;
	const currentPage = Math.floor(offset / pageSize) + 1;
	const totalPages = Math.max(1, Math.ceil(state.total / pageSize));
	const pageSizeOptions = EXTERNAL_AUTH_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const createSteps: ExternalAuthCreateStep[] = useMemo(
		() => [
			{
				title: t("admin.externalAuth.steps.type.title"),
				description: t("admin.externalAuth.steps.type.description"),
			},
			{
				title: t("admin.externalAuth.steps.connection.title"),
				description: t("admin.externalAuth.steps.connection.description"),
			},
			{
				title: t("admin.externalAuth.steps.review.title"),
				description: t("admin.externalAuth.steps.review.description"),
			},
		],
		[t],
	);

	if (state.createStep !== previousCreateStepRef.current) {
		stepAnimationRef.current =
			state.createStep > previousCreateStepRef.current ? "forward" : "backward";
		previousCreateStepRef.current = state.createStep;
	}

	const updatePagination = useCallback(
		({
			offset: nextOffset = offset,
			pageSize: nextPageSize = pageSize,
		}: {
			offset?: number;
			pageSize?: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
		}) => {
			const next = new URLSearchParams(searchParams);
			if (nextOffset > 0) next.set("offset", String(nextOffset));
			else next.delete("offset");
			if (nextPageSize !== DEFAULT_EXTERNAL_AUTH_PAGE_SIZE) {
				next.set("pageSize", String(nextPageSize));
			} else {
				next.delete("pageSize");
			}
			if (next.toString() !== searchParams.toString()) {
				setSearchParams(next, { replace: true });
			}
		},
		[offset, pageSize, searchParams, setSearchParams],
	);

	const loadProviders = useCallback(async () => {
		try {
			dispatch({ type: "set_loading", value: true });
			const [providerKinds, page] = await Promise.all([
				adminExternalAuthService.kinds(),
				adminExternalAuthService.list({ limit: pageSize, offset }),
			]);
			if (page.items.length === 0 && page.total > 0 && offset > 0) {
				updatePagination({
					offset: Math.max(
						0,
						Math.floor((page.total - 1) / pageSize) * pageSize,
					),
				});
				return;
			}
			dispatch({
				type: "loaded",
				providerKinds,
				providers: page.items,
				total: page.total,
			});
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_loading", value: false });
		}
	}, [offset, pageSize, updatePagination]);

	useEffect(() => {
		void loadProviders();
	}, [loadProviders]);

	const openCreate = useCallback(() => {
		const firstKind = state.providerKinds[0];
		dispatch({
			type: "open_create",
			form: {
				...emptyExternalAuthForm,
				providerKind: firstKind?.kind ?? "oidc",
				scopes: defaultScopesForKind(firstKind),
			},
		});
	}, [state.providerKinds]);

	function openEdit(provider: AdminExternalAuthProviderInfo) {
		dispatch({
			type: "open_edit",
			form: formFromProvider(provider),
			provider,
		});
	}

	function setField<K extends keyof ExternalAuthProviderFormData>(
		key: K,
		value: ExternalAuthProviderFormData[K],
	) {
		dispatch({ type: "set_form_field", key, value });
	}

	function setProviderKind(kind: ExternalAuthKind) {
		const descriptor = state.providerKinds.find((item) => item.kind === kind);
		dispatch({
			type: "set_provider_kind",
			kind,
			scopes: defaultScopesForKind(descriptor),
		});
	}

	function goCreateNext() {
		dispatch({ type: "set_create_step_touched", value: true });
		if (
			state.createStep === 1 &&
			requiredFieldsMissing(state.form, selectedKind)
		) {
			return;
		}
		dispatch({ type: "set_create_step", value: state.createStep + 1 });
	}

	function copyCallback(value: string) {
		void navigator.clipboard
			.writeText(value)
			.then(() => toast.success(t("common.copied")))
			.catch(() => toast.error(t("errors.unexpected_error")));
	}

	async function testDraft() {
		try {
			dispatch({ type: "set_testing_draft", value: true });
			const result = await adminExternalAuthService.testParams(
				testParamsPayload(state.form, selectedKind),
			);
			dispatch({ type: "set_test_result", value: formatTestResult(t, result) });
			toast.success(t("admin.externalAuth.testComplete"));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_testing_draft", value: false });
		}
	}

	async function testProvider(provider: AdminExternalAuthProviderInfo) {
		try {
			dispatch({ type: "set_testing_id", value: provider.id });
			const result = await adminExternalAuthService.test(provider.id);
			toast.success(formatTestResult(t, result));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_testing_id", value: null });
		}
	}

	async function submitProvider() {
		if (requiredFieldsMissing(state.form, selectedKind)) {
			dispatch({ type: "set_create_step_touched", value: true });
			return;
		}
		try {
			dispatch({ type: "set_submitting", value: true });
			if (state.editingProvider) {
				const updated = await adminExternalAuthService.update(
					state.editingProvider.id,
					updatePayload(state.form, selectedKind),
				);
				dispatch({ type: "provider_updated", value: updated });
				dispatch({ type: "close_dialog" });
				toast.success(t("admin.externalAuth.updated"));
				return;
			}
			const created = await adminExternalAuthService.create(
				createPayload(state.form, selectedKind),
			);
			dispatch({ type: "close_dialog" });
			dispatch({ type: "set_created_provider", value: created });
			toast.success(t("admin.externalAuth.created"));
			await loadProviders();
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_submitting", value: false });
		}
	}

	async function deleteProvider() {
		const provider = state.deletingProvider;
		if (!provider) return;
		try {
			await adminExternalAuthService.delete(provider.id);
			dispatch({ type: "set_deleting_provider", value: null });
			toast.success(t("admin.externalAuth.deleted"));
			await loadProviders();
		} catch (error) {
			handleApiError(error);
		}
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.externalAuth.title")}
				description={t("admin.externalAuth.description")}
				actions={
					<ExternalAuthPageActions
						loading={state.loading}
						onCreate={openCreate}
						onRefresh={() => void loadProviders()}
					/>
				}
			/>

			<ExternalAuthProviderList
				currentPage={currentPage}
				offset={offset}
				pageSize={pageSize}
				pageSizeOptions={pageSizeOptions}
				state={state}
				totalPages={totalPages}
				onCopyCallbackUrl={copyCallback}
				onCreate={openCreate}
				onEdit={openEdit}
				onRequestDelete={(provider) =>
					dispatch({ type: "set_deleting_provider", value: provider })
				}
				onTestProvider={(provider) => void testProvider(provider)}
				onUpdatePagination={updatePagination}
			/>

			<ExternalAuthPageDialogs
				createStepDirection={stepAnimationRef.current}
				createSteps={createSteps}
				state={state}
				onCloseCreatedProvider={() =>
					dispatch({ type: "set_created_provider", value: null })
				}
				onCloseDeleteProvider={() =>
					dispatch({ type: "set_deleting_provider", value: null })
				}
				onCloseProviderDialog={() => dispatch({ type: "close_dialog" })}
				onCopyCallbackUrl={copyCallback}
				onCreateBack={() =>
					dispatch({ type: "set_create_step", value: state.createStep - 1 })
				}
				onCreateNext={goCreateNext}
				onCreateStepChange={(value) =>
					dispatch({ type: "set_create_step", value })
				}
				onDeleteProvider={() => void deleteProvider()}
				onFieldChange={setField}
				onProviderKindChange={setProviderKind}
				onSubmit={() => void submitProvider()}
				onTestConnection={() => void testDraft()}
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
			<Button
				type="button"
				variant="outline"
				size="sm"
				disabled={loading}
				onClick={onRefresh}
			>
				<Icon
					name={loading ? "Spinner" : "ArrowsClockwise"}
					className={cn("mr-2 size-4", loading && "animate-spin")}
				/>
				{t("common.refresh")}
			</Button>
		</>
	);
}

function ExternalAuthProviderList({
	currentPage,
	offset,
	onCopyCallbackUrl,
	onCreate,
	onEdit,
	onRequestDelete,
	onTestProvider,
	onUpdatePagination,
	pageSize,
	pageSizeOptions,
	state,
	totalPages,
}: {
	currentPage: number;
	offset: number;
	onCopyCallbackUrl: (value: string) => void;
	onCreate: () => void;
	onEdit: (provider: AdminExternalAuthProviderInfo) => void;
	onRequestDelete: (provider: AdminExternalAuthProviderInfo) => void;
	onTestProvider: (provider: AdminExternalAuthProviderInfo) => void;
	onUpdatePagination: (value: {
		offset?: number;
		pageSize?: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
	}) => void;
	pageSize: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
	pageSizeOptions: { label: string; value: string }[];
	state: UiState;
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
				total={state.total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={offset === 0}
				nextDisabled={offset + pageSize >= state.total}
				onPrevious={() =>
					onUpdatePagination({ offset: Math.max(0, offset - pageSize) })
				}
				onNext={() => onUpdatePagination({ offset: offset + pageSize })}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(
						value,
						EXTERNAL_AUTH_PAGE_SIZE_OPTIONS,
					);
					if (next == null) return;
					onUpdatePagination({ offset: 0, pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			offset,
			onUpdatePagination,
			pageSize,
			pageSizeOptions,
			state.total,
			totalPages,
		],
	);

	return (
		<AdminTableList
			loading={state.loading}
			items={state.providers}
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
	createStepDirection,
	createSteps,
	onCloseCreatedProvider,
	onCloseDeleteProvider,
	onCloseProviderDialog,
	onCopyCallbackUrl,
	onCreateBack,
	onCreateNext,
	onCreateStepChange,
	onDeleteProvider,
	onFieldChange,
	onProviderKindChange,
	onSubmit,
	onTestConnection,
	state,
}: {
	createStepDirection: "idle" | "forward" | "backward";
	createSteps: ExternalAuthCreateStep[];
	onCloseCreatedProvider: () => void;
	onCloseDeleteProvider: () => void;
	onCloseProviderDialog: () => void;
	onCopyCallbackUrl: (value: string) => void;
	onCreateBack: () => void;
	onCreateNext: () => void;
	onCreateStepChange: (value: number) => void;
	onDeleteProvider: () => void;
	onFieldChange: <K extends keyof ExternalAuthProviderFormData>(
		key: K,
		value: ExternalAuthProviderFormData[K],
	) => void;
	onProviderKindChange: (kind: ExternalAuthKind) => void;
	onSubmit: () => void;
	onTestConnection: () => void;
	state: UiState;
}) {
	const { t } = useTranslation();

	return (
		<>
			<ExternalAuthProviderDialog
				createStep={state.createStep}
				createStepDirection={createStepDirection}
				createStepTouched={state.createStepTouched}
				createSteps={createSteps}
				form={state.form}
				mode={state.editingProvider ? "edit" : "create"}
				open={state.dialogOpen}
				provider={state.editingProvider}
				providerKinds={state.providerKinds}
				submitting={state.submitting}
				testResult={state.testResult}
				testing={state.testingDraft}
				onCopyCallbackUrl={onCopyCallbackUrl}
				onCreateBack={onCreateBack}
				onCreateNext={onCreateNext}
				onCreateStepChange={onCreateStepChange}
				onFieldChange={onFieldChange}
				onOpenChange={(open) => {
					if (!open) onCloseProviderDialog();
				}}
				onProviderKindChange={onProviderKindChange}
				onSubmit={onSubmit}
				onTestConnection={onTestConnection}
			/>

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
						onCopyCallbackUrl(callbackUrl(state.createdProvider));
					}
				}}
			/>
		</>
	);
}

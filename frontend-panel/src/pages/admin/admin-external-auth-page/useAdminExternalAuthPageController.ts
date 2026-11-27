import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { handleApiError } from "@/hooks/useApiError";
import { useClipboard } from "@/hooks/useClipboard";
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedPageSizeField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePageTitle } from "@/hooks/usePageTitle";
import { adminExternalAuthService } from "@/services/adminService";
import type {
	AdminExternalAuthProviderInfo,
	AdminExternalAuthProviderPage,
	ExternalAuthKind,
	ExternalAuthProviderKindInfo,
} from "@/types/api";
import {
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
} from "./shared";

type ProviderCursor = NonNullable<AdminExternalAuthProviderPage["next_cursor"]>;

export type ExternalAuthUiState = {
	createStep: number;
	createStepTouched: boolean;
	createdProvider: AdminExternalAuthProviderInfo | null;
	deletingProvider: AdminExternalAuthProviderInfo | null;
	/**
	 * Incremented on every dialog open. Save completions carry the session
	 * they belong to, so a slow save for dialog A can never close (or wipe)
	 * a newer dialog B opened after the admin dismissed A's dialog.
	 */
	dialogSession: number;
	dialogOpen: boolean;
	editingProvider: AdminExternalAuthProviderInfo | null;
	form: ExternalAuthProviderFormData;
	providerKinds: ExternalAuthProviderKindInfo[];
	submitting: boolean;
	testResult: string | null;
	testingDraft: boolean;
	testingId: number | null;
};

type UiAction =
	| {
			type: "loaded_kinds";
			providerKinds: ExternalAuthProviderKindInfo[];
	  }
	| { type: "open_create"; form: ExternalAuthProviderFormData }
	| {
			type: "open_edit";
			form: ExternalAuthProviderFormData;
			provider: AdminExternalAuthProviderInfo;
	  }
	| { type: "close_dialog"; session?: number }
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
	| { type: "save_settled"; session: number }
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
	  };

function initialState(): ExternalAuthUiState {
	return {
		createStep: 0,
		createStepTouched: false,
		createdProvider: null,
		deletingProvider: null,
		dialogSession: 0,
		dialogOpen: false,
		editingProvider: null,
		form: emptyExternalAuthForm,
		providerKinds: [],
		submitting: false,
		testResult: null,
		testingDraft: false,
		testingId: null,
	};
}

function reducer(
	state: ExternalAuthUiState,
	action: UiAction,
): ExternalAuthUiState {
	switch (action.type) {
		case "loaded_kinds":
			return {
				...state,
				providerKinds: sortExternalAuthProviderKinds(action.providerKinds),
			};
		case "open_create":
			return {
				...state,
				createStep: 0,
				createStepTouched: false,
				dialogOpen: true,
				dialogSession: state.dialogSession + 1,
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
				dialogSession: state.dialogSession + 1,
				editingProvider: action.provider,
				form: action.form,
				testResult: null,
			};
		case "close_dialog": {
			// A session-tagged close only applies to the dialog it was issued
			// for; the header close button may have dismissed dialog A and a
			// newer dialog B may be open with fresh drafts.
			if (
				action.session !== undefined &&
				action.session !== state.dialogSession
			) {
				return { ...state, submitting: false };
			}
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
		}
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
					microsoftTenant: "common",
					microsoftTenantMode: "common",
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
		case "save_settled":
			// A late save completion must not clear the submitting flag of a
			// newer dialog session's own in-flight save.
			return state.dialogSession === action.session
				? { ...state, submitting: false }
				: state;
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
	}
}

export function useAdminExternalAuthPageController() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.externalAuth.title"));

	const { query, setQuery } = useManagedListQueryState<{
		pageSize: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
	}>({
		schema: {
			pageSize: managedPageSizeField(
				EXTERNAL_AUTH_PAGE_SIZE_OPTIONS,
				DEFAULT_EXTERNAL_AUTH_PAGE_SIZE,
			),
		},
		searchParams,
		setSearchParams,
	});
	const pageSize = query.pageSize;
	const [state, dispatch] = useReducer(reducer, undefined, initialState);
	const { copy } = useClipboard();
	const previousCreateStepRef = useRef(0);
	const stepAnimationRef = useRef<"idle" | "forward" | "backward">("idle");
	const selectedKind =
		state.providerKinds.find((kind) => kind.kind === state.form.providerKind) ??
		state.providerKinds[0] ??
		null;
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

	const loadProviderPage = useCallback(
		async ({ cursor }: { cursor: ProviderCursor | null }) => {
			const page = await adminExternalAuthService.list({
				limit: pageSize,
				after_display_name: cursor?.value,
				after_id: cursor?.id,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[pageSize],
	);
	const list = useCursorList<AdminExternalAuthProviderInfo, ProviderCursor>({
		deps: [pageSize],
		loadPage: loadProviderPage,
		onError: handleApiError,
	});
	const { currentPage } = list;
	const totalPages = Math.max(currentPage, Math.ceil(list.total / pageSize));

	// Provider kinds are static metadata for the whole page; load them once.
	useEffect(() => {
		let cancelled = false;
		adminExternalAuthService
			.kinds()
			.then((providerKinds) => {
				if (!cancelled) {
					dispatch({ type: "loaded_kinds", providerKinds });
				}
			})
			.catch((error) => {
				if (!cancelled) {
					handleApiError(error);
				}
			});
		return () => {
			cancelled = true;
		};
	}, []);

	if (state.createStep !== previousCreateStepRef.current) {
		stepAnimationRef.current =
			state.createStep > previousCreateStepRef.current ? "forward" : "backward";
		previousCreateStepRef.current = state.createStep;
	}

	const updatePagination = useCallback(
		({
			pageSize: nextPageSize = pageSize,
		}: {
			pageSize?: (typeof EXTERNAL_AUTH_PAGE_SIZE_OPTIONS)[number];
		}) => {
			// pageSize is a list dep: the cursor list resets to the first page.
			setQuery({ pageSize: nextPageSize });
		},
		[pageSize, setQuery],
	);

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

	async function copyCallback(value: string) {
		if (await copy(value)) {
			toast.success(t("common.copied"));
		} else {
			toast.error(t("errors.unexpected_error"));
		}
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
		// Tag the in-flight save with this dialog session: if the admin closes
		// the dialog mid-save and opens another one, the late completion must
		// not close the new session or discard its drafts.
		const session = state.dialogSession;
		try {
			dispatch({ type: "set_submitting", value: true });
			if (state.editingProvider) {
				const updated = await adminExternalAuthService.update(
					state.editingProvider.id,
					updatePayload(state.form, selectedKind),
				);
				list.setItems((current) =>
					current.map((provider) =>
						provider.id === updated.id ? updated : provider,
					),
				);
				dispatch({ type: "close_dialog", session });
				toast.success(t("admin.externalAuth.updated"));
				return;
			}
			const created = await adminExternalAuthService.create(
				createPayload(state.form, selectedKind),
			);
			dispatch({ type: "close_dialog", session });
			dispatch({ type: "set_created_provider", value: created });
			toast.success(t("admin.externalAuth.created"));
			await list.reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "save_settled", session });
		}
	}

	async function deleteProvider() {
		const provider = state.deletingProvider;
		if (!provider) return;
		try {
			await adminExternalAuthService.delete(provider.id);
			dispatch({ type: "set_deleting_provider", value: null });
			toast.success(t("admin.externalAuth.deleted"));
			await list.reload();
		} catch (error) {
			handleApiError(error);
		}
	}

	return {
		createSteps,
		currentPage,
		dispatch,
		list,
		openCreate,
		openEdit,
		pageSize,
		pageSizeOptions,
		state,
		stepAnimationRef,
		totalPages,
		updatePagination,
		actions: {
			copyCallback,
			deleteProvider,
			goCreateNext,
			setField,
			setProviderKind,
			submitProvider,
			testDraft,
			testProvider,
		},
	};
}

export type AdminExternalAuthPageController = ReturnType<
	typeof useAdminExternalAuthPageController
>;

import { useEffect, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { usePageTitle } from "@/hooks/usePageTitle";
import { adminSettingsCategoryPath } from "@/routes/routePaths";
import { adminConfigService } from "@/services/adminService";
import type {
	ConfigSchemaItem,
	SystemConfig,
	SystemConfigPage,
	TemplateVariableGroup,
} from "@/types/api";
import {
	categoryOrder,
	formatError,
	isKnownCategory,
	normalizeRouteCategory,
	rootCategory,
	sortConfigs,
} from "./settingsCategories";
import {
	buildSetConfigRequest,
	type DraftValue,
	draftEqualsConfig,
	draftsFromConfigs,
	draftToValue,
	type ValidationIssue,
	validateDraft,
} from "./settingsDraft";

const CONFIG_PAGE_SIZE = 100;

export type AdminSettingsState = {
	activeTemplateVariableGroupCode: string | null;
	configs: SystemConfig[];
	drafts: Record<string, DraftValue>;
	expandedTemplateGroups: Record<string, boolean>;
	loading: boolean;
	rotatingYggdrasilKey: boolean;
	savedAt: string | null;
	saveError: string | null;
	saving: boolean;
	schema: ConfigSchemaItem[];
	sendingTestEmail: boolean;
	templateVariableGroups: TemplateVariableGroup[];
	testEmailDialogOpen: boolean;
	testEmailTarget: string;
};

export type AdminSettingsAction =
	| {
			type: "loaded";
			configs: SystemConfig[];
			schema: ConfigSchemaItem[];
			templateVariableGroups: TemplateVariableGroup[];
	  }
	| { type: "load_finished" }
	| { type: "set_active_template_variable_group_code"; value: string | null }
	| { type: "set_draft"; key: string; draft: DraftValue }
	| { type: "discard_changes" }
	| { type: "set_saving"; value: boolean }
	| { type: "save_failed"; message: string }
	| {
			type: "saved";
			/**
			 * Draft value each submitted key carried at submit time. Keys the
			 * user edited while the request was in flight keep their draft;
			 * the server result only becomes the config baseline.
			 */
			submittedDrafts: Record<string, DraftValue>;
			updated: SystemConfig[];
	  }
	| { type: "reloaded"; configs: SystemConfig[] }
	| { type: "set_test_email_dialog_open"; value: boolean }
	| { type: "set_test_email_target"; value: string }
	| { type: "set_sending_test_email"; value: boolean }
	| { type: "set_rotating_yggdrasil_key"; value: boolean }
	| { type: "set_saved_at"; value: string | null }
	| { type: "toggle_template_group"; groupKey: string; open: boolean };

const initialAdminSettingsState: AdminSettingsState = {
	activeTemplateVariableGroupCode: null,
	configs: [],
	drafts: {},
	expandedTemplateGroups: {},
	loading: true,
	rotatingYggdrasilKey: false,
	savedAt: null,
	saveError: null,
	saving: false,
	schema: [],
	sendingTestEmail: false,
	templateVariableGroups: [],
	testEmailDialogOpen: false,
	testEmailTarget: "",
};

async function loadAllConfigs() {
	const items: SystemConfig[] = [];
	let cursor: SystemConfigPage["next_cursor"] | undefined;
	let latestPage: SystemConfigPage | null = null;

	do {
		latestPage = await adminConfigService.list({
			limit: CONFIG_PAGE_SIZE,
			after_id: cursor?.id,
		});
		items.push(...latestPage.items);
		cursor = latestPage.next_cursor;
	} while (cursor);

	return items;
}

export function adminSettingsReducer(
	state: AdminSettingsState,
	action: AdminSettingsAction,
): AdminSettingsState {
	switch (action.type) {
		case "loaded": {
			const configs = sortConfigs(action.configs);
			return {
				...state,
				configs,
				drafts: draftsFromConfigs(configs),
				loading: false,
				schema: action.schema,
				templateVariableGroups: action.templateVariableGroups,
			};
		}
		case "load_finished":
			return { ...state, loading: false };
		case "set_active_template_variable_group_code":
			return { ...state, activeTemplateVariableGroupCode: action.value };
		case "set_draft":
			return {
				...state,
				drafts: { ...state.drafts, [action.key]: action.draft },
				savedAt: null,
				saveError: null,
			};
		case "discard_changes":
			return {
				...state,
				drafts: draftsFromConfigs(state.configs),
				savedAt: null,
				saveError: null,
			};
		case "set_saving":
			return {
				...state,
				saving: action.value,
				saveError: action.value ? null : state.saveError,
			};
		case "save_failed":
			return { ...state, saveError: action.message };
		case "saved": {
			const updated = action.updated;
			const configs = sortConfigs(
				state.configs.map((config) => {
					const next = updated.find((result) => result.key === config.key);
					return next ?? config;
				}),
			);
			// The config baseline always adopts the server result (so the save
			// bar can diff it against a further-edited draft), but a draft the
			// user edited while the request was in flight survives untouched:
			// submitting B and retyping C must not let the response restore B.
			const serverDrafts = draftsFromConfigs(updated);
			const drafts = { ...state.drafts };
			for (const [key, serverDraft] of Object.entries(serverDrafts)) {
				const submitted = action.submittedDrafts[key];
				if (submitted === undefined || state.drafts[key] === submitted) {
					drafts[key] = serverDraft;
				}
			}
			return {
				...state,
				configs,
				drafts,
				savedAt: new Date().toISOString(),
			};
		}
		case "reloaded": {
			const configs = sortConfigs(action.configs);
			return {
				...state,
				configs,
				drafts: draftsFromConfigs(configs),
			};
		}
		case "set_test_email_dialog_open":
			return { ...state, testEmailDialogOpen: action.value };
		case "set_test_email_target":
			return { ...state, testEmailTarget: action.value };
		case "set_sending_test_email":
			return { ...state, sendingTestEmail: action.value };
		case "set_rotating_yggdrasil_key":
			return { ...state, rotatingYggdrasilKey: action.value };
		case "set_saved_at":
			return { ...state, savedAt: action.value };
		case "toggle_template_group":
			return {
				...state,
				expandedTemplateGroups: {
					...state.expandedTemplateGroups,
					[action.groupKey]: action.open,
				},
			};
	}
}

export function useAdminSettingsPageController() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const { category: routeCategory } = useParams<{ category?: string }>();
	const [state, dispatch] = useReducer(
		adminSettingsReducer,
		initialAdminSettingsState,
	);
	const {
		activeTemplateVariableGroupCode,
		configs,
		drafts,
		expandedTemplateGroups,
		loading,
		rotatingYggdrasilKey,
		savedAt,
		saveError,
		saving,
		schema,
		sendingTestEmail,
		templateVariableGroups,
		testEmailDialogOpen,
		testEmailTarget,
	} = state;

	usePageTitle(t("settings_title"));

	useEffect(() => {
		let cancelled = false;
		Promise.all([
			loadAllConfigs(),
			adminConfigService.schema(),
			adminConfigService.templateVariables(),
		])
			.then(([configs, nextSchema, nextTemplateVariableGroups]) => {
				if (cancelled) return;
				dispatch({
					type: "loaded",
					configs,
					schema: nextSchema,
					templateVariableGroups: nextTemplateVariableGroups,
				});
			})
			.catch((nextError: unknown) => {
				if (cancelled) return;
				toast.error(formatError(nextError));
			})
			.finally(() => {
				if (!cancelled) dispatch({ type: "load_finished" });
			});
		return () => {
			cancelled = true;
		};
	}, []);

	const schemaMap = useMemo(
		() => new Map(schema.map((item) => [item.key, item])),
		[schema],
	);
	const categories = useMemo(() => {
		const present = new Set(
			configs.map((config) => rootCategory(config.category)),
		);
		return categoryOrder.filter((category) => present.has(category));
	}, [configs]);
	const requestedCategory = normalizeRouteCategory(routeCategory);
	const fallbackCategory = categories.includes("site")
		? "site"
		: (categories[0] ?? "site");
	const active =
		requestedCategory && isKnownCategory(requestedCategory)
			? requestedCategory
			: fallbackCategory;

	useEffect(() => {
		if (loading || categories.length === 0) return;
		if (requestedCategory == null) {
			navigate(adminSettingsCategoryPath(fallbackCategory), { replace: true });
			return;
		}
		if (
			!categories.includes(requestedCategory as (typeof categoryOrder)[number])
		) {
			navigate(adminSettingsCategoryPath(fallbackCategory), { replace: true });
		}
	}, [categories, fallbackCategory, loading, navigate, requestedCategory]);

	const filteredConfigs = useMemo(() => {
		return configs.filter((config) => {
			return rootCategory(config.category) === active;
		});
	}, [active, configs]);
	const groupedConfigs = useMemo(
		() =>
			filteredConfigs.reduce<Record<string, SystemConfig[]>>(
				(groups, config) => {
					groups[config.category] = groups[config.category] ?? [];
					groups[config.category].push(config);
					return groups;
				},
				{},
			),
		[filteredConfigs],
	);
	const changedConfigs = useMemo(
		() =>
			configs.filter((config) => {
				const draft = drafts[config.key];
				return draft ? !draftEqualsConfig(config, draft) : false;
			}),
		[configs, drafts],
	);
	const validationIssues = useMemo(
		() =>
			changedConfigs
				.map((config) =>
					validateDraft(config, drafts[config.key], (messageKey) =>
						t(messageKey),
					),
				)
				.filter((issue): issue is ValidationIssue => Boolean(issue)),
		[changedConfigs, drafts, t],
	);
	const activeTemplateVariableGroup = useMemo(
		() =>
			activeTemplateVariableGroupCode
				? (templateVariableGroups.find(
						(group) => group.template_code === activeTemplateVariableGroupCode,
					) ?? null)
				: null,
		[activeTemplateVariableGroupCode, templateVariableGroups],
	);

	function updateDraft(key: string, draft: DraftValue) {
		dispatch({ type: "set_draft", key, draft });
	}

	function discardChanges() {
		dispatch({ type: "discard_changes" });
	}

	async function saveChanges() {
		if (validationIssues.length > 0) return;
		dispatch({ type: "set_saving", value: true });
		try {
			// Partial success is a real outcome: A may persist while B fails.
			// Commit A's result so it is not re-submitted, keep B's draft for
			// correction, and only settle `saving` after every request ended —
			// an early release would let a retry overlap the still-running A.
			const submittedDrafts: Record<string, DraftValue> = {};
			const settled = await Promise.allSettled(
				changedConfigs.map((config) => {
					const draft = drafts[config.key];
					submittedDrafts[config.key] = draft;
					return adminConfigService.set(
						config.key,
						buildSetConfigRequest(
							config,
							draftToValue(config.value_type, draft),
						),
					);
				}),
			);
			const fulfilled = settled.filter(
				(
					item,
				): item is PromiseFulfilledResult<
					Awaited<ReturnType<typeof adminConfigService.set>>
				> => item.status === "fulfilled",
			);
			const rejected = settled.filter(
				(item): item is PromiseRejectedResult => item.status === "rejected",
			);
			if (fulfilled.length > 0) {
				dispatch({
					type: "saved",
					submittedDrafts,
					updated: fulfilled.map((item) => item.value.config),
				});
				for (const warning of fulfilled.flatMap(
					(item) => item.value.warnings,
				)) {
					if (warning.message) toast.warning(warning.message);
				}
			}
			if (rejected.length > 0) {
				const last = rejected[rejected.length - 1].reason;
				dispatch({
					type: "save_failed",
					message: `${formatError(last)} (${rejected.length}/${settled.length})`,
				});
			}
		} catch (nextError) {
			dispatch({ type: "save_failed", message: formatError(nextError) });
		} finally {
			dispatch({ type: "set_saving", value: false });
		}
	}

	async function reloadConfigs() {
		const configs = await loadAllConfigs();
		dispatch({ type: "reloaded", configs });
	}

	async function sendTestEmail() {
		dispatch({ type: "set_sending_test_email", value: true });
		try {
			const result = await adminConfigService.sendTestEmail(testEmailTarget);
			toast.success(result.message || t("mail_test_email_sent_default"));
			dispatch({ type: "set_test_email_dialog_open", value: false });
		} catch (nextError) {
			toast.error(formatError(nextError));
		} finally {
			dispatch({ type: "set_sending_test_email", value: false });
		}
	}

	async function rotateYggdrasilSignatureKey() {
		dispatch({ type: "set_rotating_yggdrasil_key", value: true });
		try {
			const result = await adminConfigService.rotateYggdrasilSignatureKey();
			toast.success(
				result.message || t("yggdrasil_rotate_signature_key_success"),
			);
			await reloadConfigs();
			dispatch({ type: "set_saved_at", value: new Date().toISOString() });
		} catch (nextError) {
			toast.error(formatError(nextError));
		} finally {
			dispatch({ type: "set_rotating_yggdrasil_key", value: false });
		}
	}

	return {
		active,
		activeTemplateVariableGroup,
		activeTemplateVariableGroupCode,
		categories,
		changedCount: changedConfigs.length,
		configs,
		disabled: validationIssues.length > 0,
		dispatch,
		drafts,
		empty: filteredConfigs.length === 0,
		error: saveError ?? validationIssues[0]?.message ?? null,
		expandedTemplateGroups,
		groupedConfigs,
		loading,
		rotatingYggdrasilKey,
		savedAt,
		saving,
		schemaMap,
		sendingTestEmail,
		testEmailDialogOpen,
		testEmailTarget,
		onDiscard: discardChanges,
		onRotateYggdrasilSignatureKey: () => void rotateYggdrasilSignatureKey(),
		onSave: () => void saveChanges(),
		onSendTestEmail: () => void sendTestEmail(),
		onUpdateDraft: updateDraft,
	};
}

export type AdminSettingsPageController = ReturnType<
	typeof useAdminSettingsPageController
>;

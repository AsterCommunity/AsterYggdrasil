import type { FormEvent } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { handleApiError } from "@/hooks/useApiError";
import { useCursorList } from "@/hooks/useCursorList";
import { usePageTitle } from "@/hooks/usePageTitle";
import { parsePageSizeSearchParam } from "@/lib/pagination";
import { adminYggdrasilSessionForwardService } from "@/services/adminService";
import type {
	AdminYggdrasilSessionForwardServerInfo,
	AdminYggdrasilSessionForwardServerPage,
	UpdateYggdrasilSessionForwardServerRequest,
} from "@/types/api";
import {
	createPayload,
	DEFAULT_PAGE_SIZE,
	DEFAULT_SORT,
	emptyForm,
	type ForwardForm,
	type ForwardSort,
	formFromServer,
	PAGE_SIZE_OPTIONS,
	parseForwardSort,
	SORT_OPTIONS,
	updatePayload,
} from "./forwardServerForm";

type ForwardCursor = NonNullable<
	AdminYggdrasilSessionForwardServerPage["next_cursor"]
>;

const DIALOG_EXIT_ANIMATION_MS = 160;

export function useAdminYggdrasilForwardingPageController() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();
	usePageTitle(t("admin.yggdrasilForwarding.title"));

	const pageSize = parsePageSizeSearchParam(
		searchParams.get("pageSize"),
		PAGE_SIZE_OPTIONS,
		DEFAULT_PAGE_SIZE,
	);
	const sort = parseForwardSort(searchParams.get("sort_by"));
	const [dialogOpen, setDialogOpen] = useState(false);
	const [editingServer, setEditingServer] =
		useState<AdminYggdrasilSessionForwardServerInfo | null>(null);
	const [deletingServer, setDeletingServer] =
		useState<AdminYggdrasilSessionForwardServerInfo | null>(null);
	const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
	const [form, setForm] = useState<ForwardForm>(() => emptyForm());
	const [submitting, setSubmitting] = useState(false);
	const [updatingId, setUpdatingId] = useState<number | null>(null);
	const editDialogCleanupRef = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const deleteDialogCleanupRef = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const pageSizeOptions = PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const sortOptions = useMemo(
		() =>
			SORT_OPTIONS.map((value) => ({
				label: t(`admin.yggdrasilForwarding.sort.${value}`),
				value,
			})),
		[t],
	);
	const selectedSortLabel =
		sortOptions.find((option) => option.value === sort)?.label ??
		t(`admin.yggdrasilForwarding.sort.${DEFAULT_SORT}`);

	const loadServerPage = useCallback(
		async ({ cursor }: { cursor: ForwardCursor | null }) => {
			const page = await adminYggdrasilSessionForwardService.list({
				limit: pageSize,
				sort_by: sort,
				after_enabled:
					cursor && "call_order" in cursor
						? cursor.call_order.enabled
						: undefined,
				after_priority:
					cursor && "call_order" in cursor
						? cursor.call_order.priority
						: undefined,
				after_id:
					cursor && "call_order" in cursor
						? cursor.call_order.id
						: cursor && "id" in cursor
							? cursor.id.id
							: undefined,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[pageSize, sort],
	);
	const list = useCursorList<
		AdminYggdrasilSessionForwardServerInfo,
		ForwardCursor
	>({
		deps: [pageSize, sort],
		loadPage: loadServerPage,
		onError: handleApiError,
	});
	const { currentPage } = list;
	const totalPages = Math.max(currentPage, Math.ceil(list.total / pageSize));

	const updatePagination = useCallback(
		({
			pageSize: nextPageSize = pageSize,
			sort: nextSort = sort,
		}: {
			pageSize?: (typeof PAGE_SIZE_OPTIONS)[number];
			sort?: ForwardSort;
		}) => {
			const next = new URLSearchParams(searchParams);
			if (nextPageSize !== DEFAULT_PAGE_SIZE) {
				next.set("pageSize", String(nextPageSize));
			} else {
				next.delete("pageSize");
			}
			if (nextSort !== DEFAULT_SORT) next.set("sort_by", nextSort);
			else next.delete("sort_by");
			if (next.toString() !== searchParams.toString()) {
				setSearchParams(next, { replace: true });
			}
			// Both params are list deps, so the cursor list resets itself.
		},
		[pageSize, searchParams, setSearchParams, sort],
	);

	useEffect(
		() => () => {
			if (editDialogCleanupRef.current) {
				clearTimeout(editDialogCleanupRef.current);
			}
			if (deleteDialogCleanupRef.current) {
				clearTimeout(deleteDialogCleanupRef.current);
			}
		},
		[],
	);

	function cancelEditDialogCleanup() {
		if (!editDialogCleanupRef.current) return;
		clearTimeout(editDialogCleanupRef.current);
		editDialogCleanupRef.current = null;
	}

	function cancelDeleteDialogCleanup() {
		if (!deleteDialogCleanupRef.current) return;
		clearTimeout(deleteDialogCleanupRef.current);
		deleteDialogCleanupRef.current = null;
	}

	function closeForwardServerDialog() {
		setDialogOpen(false);
		cancelEditDialogCleanup();
		editDialogCleanupRef.current = setTimeout(() => {
			setEditingServer(null);
			editDialogCleanupRef.current = null;
		}, DIALOG_EXIT_ANIMATION_MS);
	}

	function closeDeleteDialog() {
		setDeleteDialogOpen(false);
		cancelDeleteDialogCleanup();
		deleteDialogCleanupRef.current = setTimeout(() => {
			setDeletingServer(null);
			deleteDialogCleanupRef.current = null;
		}, DIALOG_EXIT_ANIMATION_MS);
	}

	function openCreate() {
		cancelEditDialogCleanup();
		setEditingServer(null);
		setForm(emptyForm());
		setDialogOpen(true);
	}

	function openEdit(server: AdminYggdrasilSessionForwardServerInfo) {
		cancelEditDialogCleanup();
		setEditingServer(server);
		setForm(formFromServer(server));
		setDialogOpen(true);
	}

	function openDelete(server: AdminYggdrasilSessionForwardServerInfo) {
		cancelDeleteDialogCleanup();
		setDeletingServer(server);
		setDeleteDialogOpen(true);
	}

	function setField<K extends keyof ForwardForm>(
		key: K,
		value: ForwardForm[K],
	) {
		setForm((current) => ({ ...current, [key]: value }));
	}

	async function submitForm(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!form.displayName.trim()) return;
		if (!editingServer && !form.baseUrl.trim()) return;

		try {
			setSubmitting(true);
			if (editingServer) {
				await adminYggdrasilSessionForwardService.update(
					editingServer.id,
					updatePayload(form, editingServer),
				);
				toast.success(t("admin.yggdrasilForwarding.updateSuccess"));
				await list.reload();
			} else {
				await adminYggdrasilSessionForwardService.create(createPayload(form));
				toast.success(t("admin.yggdrasilForwarding.createSuccess"));
				await list.reload();
			}
			closeForwardServerDialog();
		} catch (error) {
			handleApiError(error);
		} finally {
			setSubmitting(false);
		}
	}

	async function patchServer(
		server: AdminYggdrasilSessionForwardServerInfo,
		patch: UpdateYggdrasilSessionForwardServerRequest,
	) {
		try {
			setUpdatingId(server.id);
			await adminYggdrasilSessionForwardService.update(server.id, patch);
			toast.success(t("admin.yggdrasilForwarding.updateSuccess"));
			await list.reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			setUpdatingId(null);
		}
	}

	async function deleteServer() {
		if (!deletingServer) return;
		try {
			setUpdatingId(deletingServer.id);
			await adminYggdrasilSessionForwardService.delete(deletingServer.id);
			closeDeleteDialog();
			toast.success(t("admin.yggdrasilForwarding.deleteSuccess"));
			await list.reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			setUpdatingId(null);
		}
	}

	return {
		deletingServer,
		deleteDialogOpen,
		dialogOpen,
		editingServer,
		form,
		list,
		pageSize,
		pageSizeOptions,
		selectedSortLabel,
		sort,
		sortOptions,
		submitting,
		totalPages,
		updatingId,
		currentPage,
		updatePagination,
		onOpenCreate: openCreate,
		onOpenEdit: openEdit,
		onOpenDelete: openDelete,
		onPatchServer: (
			server: AdminYggdrasilSessionForwardServerInfo,
			patch: UpdateYggdrasilSessionForwardServerRequest,
		) => void patchServer(server, patch),
		onSubmitForm: (event: FormEvent<HTMLFormElement>) => void submitForm(event),
		onSetField: setField,
		onDialogOpenChange: (open: boolean) => {
			if (open) {
				cancelEditDialogCleanup();
				setDialogOpen(true);
			} else {
				closeForwardServerDialog();
			}
		},
		onDeleteDialogOpenChange: (open: boolean) => {
			if (open) {
				cancelDeleteDialogCleanup();
				setDeleteDialogOpen(true);
			} else {
				closeDeleteDialog();
			}
		},
		onDeleteServer: () => void deleteServer(),
	};
}

export type AdminYggdrasilForwardingPageController = ReturnType<
	typeof useAdminYggdrasilForwardingPageController
>;

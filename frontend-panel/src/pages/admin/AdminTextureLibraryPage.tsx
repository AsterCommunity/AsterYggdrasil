import { type FormEvent, useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { TextureLibrarySectionNav } from "@/components/admin/TextureLibrarySectionNav";
import {
	AdminTable,
	AdminTableBody,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
	AdminTableShell,
} from "@/components/common/AdminTable";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { DateTimeText } from "@/components/common/DateTimeText";
import { EmptyState } from "@/components/common/EmptyState";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { handleApiError } from "@/hooks/useApiError";
import { useConfirmDialog } from "@/hooks/useConfirmDialog";
import { useCursorList } from "@/hooks/useCursorList";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePendingAction } from "@/hooks/usePendingAction";
import { cn } from "@/lib/utils";
import { adminTextureLibraryService } from "@/services/adminService";
import type {
	CreateMinecraftTextureTagRequest,
	MinecraftTextureTagInfo,
	UpdateMinecraftTextureTagRequest,
} from "@/types/api";

type TagListResponse = Awaited<
	ReturnType<typeof adminTextureLibraryService.listTags>
>;

type TagCursor = NonNullable<TagListResponse["next_cursor"]>;

const TAG_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_TAG_PAGE_SIZE = 20;
const TAG_SKELETON_KEYS = [
	"texture-tag-skeleton-0",
	"texture-tag-skeleton-1",
	"texture-tag-skeleton-2",
	"texture-tag-skeleton-3",
	"texture-tag-skeleton-4",
	"texture-tag-skeleton-5",
] as const;

type TagFormState = {
	name: string;
	sortOrder: string;
};

function emptyTagForm(): TagFormState {
	return {
		name: "",
		sortOrder: "0",
	};
}

function tagPayload(form: TagFormState): CreateMinecraftTextureTagRequest {
	const sortOrder = Number.parseInt(form.sortOrder, 10);
	return {
		color: tagColorForName(form.name),
		name: form.name.trim(),
		sort_order: Number.isFinite(sortOrder) ? sortOrder : null,
	};
}

export default function AdminTextureLibraryPage() {
	const { t } = useTranslation();
	const [pageSize, setPageSize] = useState(DEFAULT_TAG_PAGE_SIZE);
	const { pending: submitting, runWithPending } = usePendingAction();
	const [form, setForm] = useState<TagFormState>(emptyTagForm);
	const [editingTag, setEditingTag] = useState<MinecraftTextureTagInfo | null>(
		null,
	);
	const [editForm, setEditForm] = useState<TagFormState>(emptyTagForm);
	const deleteTagDialog = useConfirmDialog<MinecraftTextureTagInfo>(
		(target) => {
			void confirmDeleteTag(target);
		},
	);

	usePageTitle(t("admin.textureLibraryPage.title"));

	const loadTagPage = useCallback(
		async ({ cursor }: { cursor: TagCursor | null }) => {
			const page = await adminTextureLibraryService.listTags({
				limit: pageSize,
				after_sort_order: cursor?.sort_order,
				after_name: cursor?.name,
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
	const list = useCursorList<MinecraftTextureTagInfo, TagCursor>({
		deps: [pageSize],
		loadPage: loadTagPage,
		onError: handleApiError,
	});
	const { items } = list;

	async function createTag(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!form.name.trim()) {
			toast.error(t("admin.textureLibraryPage.nameRequired"));
			return;
		}
		await runWithPending(async () => {
			try {
				await adminTextureLibraryService.createTag(tagPayload(form));
				setForm(emptyTagForm());
				toast.success(t("admin.textureLibraryPage.createSuccess"));
				await list.reloadFirstPage();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	function openEditDialog(tag: MinecraftTextureTagInfo) {
		setEditingTag(tag);
		setEditForm({
			name: tag.name,
			sortOrder: String(tag.sort_order),
		});
	}

	async function updateTag(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!editingTag) return;
		if (!editForm.name.trim()) {
			toast.error(t("admin.textureLibraryPage.nameRequired"));
			return;
		}
		await runWithPending(async () => {
			try {
				const payload = tagPayload(
					editForm,
				) satisfies UpdateMinecraftTextureTagRequest;
				await adminTextureLibraryService.updateTag(editingTag.id, payload);
				setEditingTag(null);
				toast.success(t("admin.textureLibraryPage.updateSuccess"));
				await list.reload();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	async function confirmDeleteTag(tag: MinecraftTextureTagInfo) {
		await runWithPending(async () => {
			try {
				await adminTextureLibraryService.deleteTag(tag.id);
				deleteTagDialog.clearConfirm();
				toast.success(t("admin.textureLibraryPage.deleteSuccess"));
				await list.reload();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	const currentPage = list.currentPage;
	const totalPages = Math.max(currentPage, Math.ceil(list.total / pageSize));
	const pageSizeOptions = TAG_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.textureLibraryPage.title")}
				description={t("admin.textureLibraryPage.description")}
				actions={
					<>
						<TextureLibrarySectionNav active="tags" />
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={list.loading || submitting}
							onClick={() => void list.reload()}
						>
							<Icon
								name={list.loading ? "Spinner" : "RefreshCw"}
								className={cn("size-4", list.loading && "animate-spin")}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
			/>

			<AdminSurface>
				<form
					className="grid gap-3 lg:grid-cols-[minmax(12rem,1fr)_10rem_8rem_auto] lg:items-end"
					onSubmit={createTag}
				>
					<div className="grid gap-1.5">
						<Label htmlFor="texture-tag-name">
							{t("admin.textureLibraryPage.name")}
						</Label>
						<Input
							id="texture-tag-name"
							value={form.name}
							maxLength={64}
							placeholder={t("admin.textureLibraryPage.namePlaceholder")}
							onChange={(event) => {
								const value = event.currentTarget.value;
								setForm((current) => ({
									...current,
									name: value,
								}));
							}}
						/>
					</div>
					<div className="grid gap-1.5">
						<div className="text-sm font-medium">
							{t("admin.textureLibraryPage.color")}
						</div>
						<div className="flex h-8 items-center gap-2 rounded-lg border border-border/70 bg-muted/25 px-2.5 text-sm">
							<span
								aria-hidden="true"
								className="size-3 rounded-full"
								style={{ backgroundColor: tagColorForName(form.name) }}
							/>
							<code>{tagColorForName(form.name)}</code>
						</div>
					</div>
					<div className="grid gap-1.5">
						<Label htmlFor="texture-tag-sort-order">
							{t("admin.textureLibraryPage.sortOrder")}
						</Label>
						<Input
							id="texture-tag-sort-order"
							type="number"
							value={form.sortOrder}
							onChange={(event) => {
								const value = event.currentTarget.value;
								setForm((current) => ({
									...current,
									sortOrder: value,
								}));
							}}
						/>
					</div>
					<Button type="submit" disabled={submitting}>
						<Icon
							name={submitting ? "Spinner" : "Plus"}
							className={cn("size-4", submitting && "animate-spin")}
						/>
						{t("admin.textureLibraryPage.createAction")}
					</Button>
				</form>
			</AdminSurface>

			<AdminTableShell>
				{list.loading ? (
					<div className="grid gap-2 p-4">
						{TAG_SKELETON_KEYS.map((key) => (
							<Skeleton key={key} className="h-11" />
						))}
					</div>
				) : items.length === 0 ? (
					<EmptyState
						icon={<Icon name="Images" className="size-5" />}
						title={t("admin.textureLibraryPage.emptyTitle")}
						description={t("admin.textureLibraryPage.emptyDescription")}
					/>
				) : (
					<AdminTable>
						<AdminTableHeader>
							<AdminTableRow>
								<AdminTableHead>
									{t("admin.textureLibraryPage.name")}
								</AdminTableHead>
								<AdminTableHead>
									{t("admin.textureLibraryPage.color")}
								</AdminTableHead>
								<AdminTableHead>
									{t("admin.textureLibraryPage.sortOrder")}
								</AdminTableHead>
								<AdminTableHead>
									{t("admin.textureLibraryPage.updatedAt")}
								</AdminTableHead>
								<AdminTableHead className="text-right">
									{t("common.actions")}
								</AdminTableHead>
							</AdminTableRow>
						</AdminTableHeader>
						<AdminTableBody>
							{items.map((tag) => (
								<AdminTableRow key={tag.id}>
									<AdminTableCell>
										<span className="inline-flex min-w-0 items-center gap-2">
											<span
												aria-hidden="true"
												className="size-2.5 rounded-full"
												style={{ backgroundColor: tag.color }}
											/>
											<span className="truncate font-medium">{tag.name}</span>
										</span>
									</AdminTableCell>
									<AdminTableCell>
										<code className="rounded bg-muted px-1.5 py-0.5 text-xs">
											{tag.color}
										</code>
									</AdminTableCell>
									<AdminTableCell>{tag.sort_order}</AdminTableCell>
									<AdminTableCell>
										<DateTimeText value={tag.updated_at} />
									</AdminTableCell>
									<AdminTableCell>
										<div className="flex justify-end gap-2">
											<Button
												type="button"
												variant="outline"
												size="sm"
												onClick={() => openEditDialog(tag)}
											>
												<Icon name="PencilSimple" className="size-4" />
												{t("admin.textureLibraryPage.editAction")}
											</Button>
											<Button
												type="button"
												variant="destructive"
												size="sm"
												onClick={() => deleteTagDialog.requestConfirm(tag)}
											>
												<Icon name="Trash" className="size-4" />
												{t("common.delete")}
											</Button>
										</div>
									</AdminTableCell>
								</AdminTableRow>
							))}
						</AdminTableBody>
					</AdminTable>
				)}
			</AdminTableShell>

			<AdminOffsetPagination
				currentPage={currentPage}
				nextDisabled={list.nextPageDisabled}
				onNext={list.goNext}
				onPageSizeChange={(value) => {
					const parsed = Number.parseInt(value ?? "", 10);
					if (TAG_PAGE_SIZE_OPTIONS.includes(parsed as never)) {
						// pageSize is a list dep, so the cursor list resets itself.
						setPageSize(parsed);
					}
				}}
				onPrevious={list.goPrevious}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={list.prevPageDisabled}
				total={list.total}
				totalPages={totalPages}
			/>

			<Dialog
				open={Boolean(editingTag)}
				onOpenChange={(open) => !open && setEditingTag(null)}
			>
				<DialogContent keepMounted className="sm:max-w-md">
					<form className="grid gap-4" onSubmit={updateTag}>
						<DialogHeader>
							<DialogTitle>
								{t("admin.textureLibraryPage.editTitle")}
							</DialogTitle>
							<DialogDescription>
								{t("admin.textureLibraryPage.editDescription")}
							</DialogDescription>
						</DialogHeader>
						<TagFields form={editForm} onChange={setEditForm} />
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								disabled={submitting}
								onClick={() => setEditingTag(null)}
							>
								{t("common.cancel")}
							</Button>
							<Button type="submit" disabled={!editingTag || submitting}>
								{t("common.save")}
							</Button>
						</DialogFooter>
					</form>
				</DialogContent>
			</Dialog>

			<ConfirmDialog
				open={deleteTagDialog.dialogProps.open}
				title={t("admin.textureLibraryPage.deleteTitle")}
				description={
					deleteTagDialog.confirmTarget
						? t("admin.textureLibraryPage.deleteDescription", {
								name: deleteTagDialog.confirmTarget.name,
							})
						: undefined
				}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				loading={submitting}
				variant="destructive"
				onOpenChange={deleteTagDialog.dialogProps.onOpenChange}
				onConfirm={deleteTagDialog.dialogProps.onConfirm}
			/>
		</AdminPageShell>
	);
}

function TagFields({
	form,
	onChange,
}: {
	form: TagFormState;
	onChange: (form: TagFormState) => void;
}) {
	const { t } = useTranslation();
	return (
		<div className="grid gap-3">
			<div className="grid gap-1.5">
				<Label htmlFor="texture-tag-edit-name">
					{t("admin.textureLibraryPage.name")}
				</Label>
				<Input
					id="texture-tag-edit-name"
					value={form.name}
					maxLength={64}
					onChange={(event) => {
						const value = event.currentTarget.value;
						onChange({ ...form, name: value });
					}}
				/>
			</div>
			<div className="grid gap-1.5">
				<div className="text-sm font-medium">
					{t("admin.textureLibraryPage.color")}
				</div>
				<div className="flex h-8 items-center gap-2 rounded-lg border border-border/70 bg-muted/25 px-2.5 text-sm">
					<span
						aria-hidden="true"
						className="size-3 rounded-full"
						style={{ backgroundColor: tagColorForName(form.name) }}
					/>
					<code>{tagColorForName(form.name)}</code>
				</div>
			</div>
			<div className="grid gap-1.5">
				<Label htmlFor="texture-tag-edit-sort-order">
					{t("admin.textureLibraryPage.sortOrder")}
				</Label>
				<Input
					id="texture-tag-edit-sort-order"
					type="number"
					value={form.sortOrder}
					onChange={(event) => {
						const value = event.currentTarget.value;
						onChange({ ...form, sortOrder: value });
					}}
				/>
			</div>
		</div>
	);
}

const TAG_COLOR_PALETTE = [
	"#2563eb",
	"#0891b2",
	"#059669",
	"#65a30d",
	"#ca8a04",
	"#dc2626",
	"#e11d48",
	"#c026d3",
	"#7c3aed",
	"#4f46e5",
	"#0d9488",
	"#ea580c",
] as const;

export function tagColorForName(name: string) {
	const normalized = name.trim().toLowerCase();
	let hash = 2_166_136_261;
	for (const character of normalized) {
		hash ^= character.codePointAt(0) ?? 0;
		hash = Math.imul(hash, 16_777_619) >>> 0;
	}
	return TAG_COLOR_PALETTE[hash % TAG_COLOR_PALETTE.length];
}

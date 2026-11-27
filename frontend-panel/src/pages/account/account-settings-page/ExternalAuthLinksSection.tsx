import { useCallback, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { RefreshButton } from "@/components/common/RefreshButton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { useCursorList } from "@/hooks/useCursorList";
import { externalAuthKindIconPath } from "@/lib/externalAuthProviders";
import { cn } from "@/lib/utils";
import { externalAuthService } from "@/services/externalAuthService";
import { formatUnknownError } from "@/services/http";
import type { DateTimeIdCursor, ExternalAuthLinkInfo } from "@/types/api";

const EXTERNAL_AUTH_LINK_PAGE_SIZE = 20;

type ExternalAuthLinksState = {
	busyId: number | null;
};

type ExternalAuthLinksAction = { type: "set_busy_id"; value: number | null };

const externalAuthLinksInitialState: ExternalAuthLinksState = {
	busyId: null,
};

function externalAuthLinksReducer(
	state: ExternalAuthLinksState,
	action: ExternalAuthLinksAction,
): ExternalAuthLinksState {
	switch (action.type) {
		case "set_busy_id":
			return { ...state, busyId: action.value };
	}
}

export function ExternalAuthLinksSection() {
	const { t } = useTranslation();
	const [state, dispatch] = useReducer(
		externalAuthLinksReducer,
		externalAuthLinksInitialState,
	);
	const { busyId } = state;

	const loadLinkPage = useCallback(
		async ({ cursor }: { cursor: DateTimeIdCursor | null }) => {
			const page = await externalAuthService.listLinksPage({
				limit: EXTERNAL_AUTH_LINK_PAGE_SIZE,
				after_created_at: cursor?.value,
				after_id: cursor?.id,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[],
	);
	const list = useCursorList<ExternalAuthLinkInfo, DateTimeIdCursor>({
		deps: [],
		loadPage: loadLinkPage,
		onError: (error: unknown) => toast.error(formatUnknownError(error)),
	});
	const { loading, total: linkTotal } = list;
	const links = list.items;

	async function unlink(link: ExternalAuthLinkInfo) {
		dispatch({ type: "set_busy_id", value: link.id });
		try {
			await externalAuthService.deleteLink(link.id);
			await list.reload();
			toast.success(t("personalSettings.externalAuthUnlinked"));
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "set_busy_id", value: null });
		}
	}

	return (
		<div className="rounded-lg border border-border/70 bg-background/55 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 border-b border-border/70 px-4 py-4 sm:flex-row sm:items-center sm:justify-between dark:border-white/10">
				<div>
					<h3 className="text-sm font-semibold">
						{t("personalSettings.externalAuthTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.externalAuthDescription")}
					</p>
				</div>
				<RefreshButton loading={loading} onClick={() => void list.reload()} />
			</div>

			<div className="divide-y divide-border/70 dark:divide-white/10">
				{loading ? (
					<div className="px-4 py-6 text-sm text-muted-foreground">
						{t("common.loading")}
					</div>
				) : links.length === 0 ? (
					<div className="px-4 py-6 text-sm text-muted-foreground">
						{t("personalSettings.externalAuthEmpty")}
					</div>
				) : (
					links.map((link) => (
						<div
							key={link.id}
							className="grid gap-3 px-4 py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
						>
							<div className="flex min-w-0 items-center gap-3">
								<img
									src={
										link.provider_icon_url ||
										externalAuthKindIconPath(link.provider_kind)
									}
									alt=""
									className="size-8 rounded-md border border-border/70 bg-background object-contain p-1 dark:border-white/10"
								/>
								<div className="min-w-0">
									<div className="truncate text-sm font-semibold">
										{link.provider_display_name}
									</div>
									<div className="mt-1 truncate text-xs text-muted-foreground">
										{link.email_snapshot ||
											link.display_name_snapshot ||
											link.subject}
									</div>
								</div>
							</div>
							<div className="flex flex-wrap items-center gap-2 sm:justify-end">
								<Badge variant="outline" className="rounded-md">
									{link.provider_kind}
								</Badge>
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={busyId === link.id}
									onClick={() => void unlink(link)}
									className="border-destructive/35 text-destructive hover:border-destructive/60 hover:bg-destructive/10 hover:text-destructive"
								>
									<Icon
										name={busyId === link.id ? "Spinner" : "Trash"}
										className={cn(
											"mr-2 size-4",
											busyId === link.id && "animate-spin",
										)}
									/>
									{t("personalSettings.externalAuthUnlink")}
								</Button>
							</div>
						</div>
					))
				)}
			</div>
			<AdminOffsetPagination
				currentPage={list.currentPage}
				nextDisabled={list.nextPageDisabled}
				onNext={list.goNext}
				onPageSizeChange={() => {}}
				onPrevious={list.goPrevious}
				pageSize={String(EXTERNAL_AUTH_LINK_PAGE_SIZE)}
				pageSizeOptions={[
					{
						label: t("admin.pagination.pageSizeOption", {
							count: EXTERNAL_AUTH_LINK_PAGE_SIZE,
						}),
						value: String(EXTERNAL_AUTH_LINK_PAGE_SIZE),
					},
				]}
				prevDisabled={list.prevPageDisabled}
				total={linkTotal}
				totalPages={list.totalPages}
			/>
		</div>
	);
}

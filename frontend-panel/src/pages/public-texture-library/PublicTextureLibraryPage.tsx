import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { PublicTextureHeaderActions } from "@/components/yggdrasil/PublicTextureHeaderActions";
import { TextureTagFilterPopover } from "@/components/yggdrasil/TextureTagFilterPopover";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePublicSession } from "@/hooks/usePublicSession";
import { cn } from "@/lib/utils";
import { PublicTextureCard } from "./PublicTextureCard";
import { TextureLibraryDisabledPanel } from "./TextureLibraryDisabledPanel";
import { usePublicTextureLibraryController } from "./usePublicTextureLibraryController";

const LIBRARY_PAGE_SIZE_OPTIONS = [12, 24] as const;
const DEFAULT_LIBRARY_PAGE_SIZE = 12;

export default function PublicTextureLibraryPage() {
	const { t } = useTranslation();
	const { isAuthenticated, logout, user } = usePublicSession();
	const controller = usePublicTextureLibraryController(
		DEFAULT_LIBRARY_PAGE_SIZE,
	);

	usePageTitle(t("library.title"));

	const {
		changePageSize,
		changeTagSearchMethod,
		clearTagFilter,
		currentPage,
		hasMoreTags,
		list,
		loading,
		pageSize,
		query,
		searchTags,
		selectedTags,
		serverName,
		setDraftTagIds,
		setQuery,
		setTagFilterOpen,
		setTextureType,
		submitSearch,
		tagFilterOpen,
		tagLoading,
		tagSearchMethod,
		textureLibraryEnabled,
		textures,
		total,
		totalPages,
	} = controller;
	const { branding, draftTagIds, loadMoreTags, tags, textureType } = controller;

	return (
		<PublicEntryShell
			branding={branding}
			title={serverName}
			tagline={t("brand.tagline")}
			variant="home"
			hideLanguageOnMobile
			headerActions={
				<PublicTextureHeaderActions
					isAuthenticated={isAuthenticated}
					user={user}
					onLogout={() => void logout()}
				/>
			}
			footer={<AppFooter />}
		>
			<main className="relative z-10 min-w-0 flex-1">
				<div className="mx-auto grid w-full max-w-[92rem] gap-5 px-4 pt-6 pb-10 sm:px-8 lg:px-12">
					<header className="py-3">
						<div className="min-w-0">
							<Badge className="rounded-full border-emerald-700/20 bg-emerald-600/12 px-3 py-1 text-emerald-800 shadow-lg shadow-black/10 dark:border-emerald-300/24 dark:bg-emerald-400/14 dark:text-emerald-100">
								<Icon name="Images" className="size-3.5" />
								{t("library.eyebrow")}
							</Badge>
							<h1 className="mt-5 max-w-4xl text-balance font-black text-4xl leading-none tracking-normal text-[#102118] sm:text-6xl dark:text-white">
								{t("library.title")}
							</h1>
							<p className="mt-4 max-w-3xl text-sm leading-6 text-slate-700 sm:text-base dark:text-slate-300">
								{t("library.description")}
							</p>
						</div>
					</header>

					{textureLibraryEnabled ? null : <TextureLibraryDisabledPanel />}

					{textureLibraryEnabled ? (
						<section className="overflow-hidden rounded-xl border border-black/10 bg-white/76 shadow-2xl shadow-emerald-950/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/25">
							<div className="flex flex-col gap-3 border-black/10 border-b bg-white/54 p-3 dark:border-white/10 dark:bg-white/[0.04] lg:flex-row lg:items-center lg:justify-between">
								<div className="flex min-w-0 flex-wrap items-center gap-1">
									<LibraryTypeButton
										active={textureType === "all"}
										onClick={() => setTextureType("all")}
									>
										{t("library.type.all")}
									</LibraryTypeButton>
									<LibraryTypeButton
										active={textureType === "skin"}
										onClick={() => setTextureType("skin")}
									>
										{t("wardrobe.type.skin")}
									</LibraryTypeButton>
									<LibraryTypeButton
										active={textureType === "cape"}
										onClick={() => setTextureType("cape")}
									>
										{t("wardrobe.type.cape")}
									</LibraryTypeButton>
								</div>
								<form
									className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center"
									onSubmit={submitSearch}
								>
									<div className="relative sm:w-80">
										<Icon
											name="MagnifyingGlass"
											aria-hidden="true"
											className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
										/>
										<Input
											value={query}
											placeholder={t("library.searchPlaceholder")}
											className="bg-white/82 pl-8 dark:bg-white/8"
											onChange={(event) => setQuery(event.currentTarget.value)}
										/>
									</div>
									<TextureTagFilterPopover
										open={tagFilterOpen}
										searchMethod={tagSearchMethod}
										selectedIds={draftTagIds}
										selectedTags={selectedTags}
										hasMore={hasMoreTags}
										loading={tagLoading}
										tags={tags}
										testId="library-tag-filter-popover"
										onClear={clearTagFilter}
										onLoadMore={loadMoreTags}
										onOpenChange={setTagFilterOpen}
										onSearchQueryChange={searchTags}
										onSearchMethodChange={changeTagSearchMethod}
										onSelectedIdsChange={setDraftTagIds}
									/>
									<Button type="submit" variant="outline" size="sm">
										{t("library.searchAction")}
									</Button>
									<Button
										type="button"
										size="sm"
										disabled={loading}
										onClick={() => void list.reload()}
									>
										{loading ? t("common.loading") : t("common.refresh")}
									</Button>
								</form>
							</div>

							<div className="min-h-[28rem] p-3">
								{loading ? (
									<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
										{Array.from(
											{ length: 8 },
											(_, index) => `texture-library-skeleton-${index}`,
										).map((key) => (
											<Skeleton key={key} className="h-72 rounded-lg" />
										))}
									</div>
								) : textures.length === 0 ? (
									<div className="grid min-h-[24rem] place-items-center rounded-lg border border-dashed border-black/15 bg-white/45 px-4 text-center dark:border-white/15 dark:bg-white/[0.04]">
										<div className="max-w-md">
											<h2 className="text-lg font-semibold tracking-normal">
												{t("library.emptyTitle")}
											</h2>
											<p className="mt-2 text-sm leading-6 text-muted-foreground">
												{t("library.emptyDescription")}
											</p>
										</div>
									</div>
								) : (
									<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
										{textures.map((texture) => (
											<PublicTextureCard key={texture.id} texture={texture} />
										))}
									</div>
								)}
							</div>

							{total > 0 ? (
								<div className="flex flex-col gap-3 border-black/10 border-t bg-white/45 px-4 py-3 dark:border-white/10 dark:bg-white/[0.035] sm:flex-row sm:items-center sm:justify-between">
									<div className="text-sm text-muted-foreground">
										{t("admin.pagination.entriesPage", {
											current: currentPage,
											pages: totalPages,
											total,
										})}
									</div>
									<div className="flex flex-wrap items-center gap-2">
										<label className="flex items-center gap-2 text-sm text-muted-foreground">
											<span>{t("admin.pagination.pageSize")}</span>
											<select
												value={pageSize}
												aria-label={t("admin.pagination.pageSize")}
												className="h-8 rounded-md border border-input bg-background px-2 text-sm"
												onChange={(event) => {
													changePageSize(Number(event.currentTarget.value));
												}}
											>
												{LIBRARY_PAGE_SIZE_OPTIONS.map((option) => (
													<option key={option} value={option}>
														{t("admin.pagination.pageSizeOption", {
															count: option,
														})}
													</option>
												))}
											</select>
										</label>
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={list.prevPageDisabled}
											onClick={list.goPrevious}
										>
											{t("admin.pagination.previous")}
										</Button>
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={list.nextPageDisabled}
											onClick={list.goNext}
										>
											{t("admin.pagination.next")}
										</Button>
									</div>
								</div>
							) : null}
						</section>
					) : null}
				</div>
			</main>
		</PublicEntryShell>
	);
}

function LibraryTypeButton({
	active,
	children,
	onClick,
}: {
	active: boolean;
	children: ReactNode;
	onClick: () => void;
}) {
	return (
		<Button
			type="button"
			variant={active ? "default" : "ghost"}
			className={cn(
				"rounded-md px-4",
				active ? "bg-primary text-primary-foreground" : "text-muted-foreground",
			)}
			onClick={onClick}
		>
			{children}
		</Button>
	);
}

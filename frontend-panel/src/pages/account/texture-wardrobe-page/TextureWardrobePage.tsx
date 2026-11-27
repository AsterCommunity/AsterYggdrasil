import { useTranslation } from "react-i18next";
import { usePageTitle } from "@/hooks/usePageTitle";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { useTextureWardrobePageController } from "./useTextureWardrobePageController";
import { WardrobeBindDialog } from "./WardrobeBindDialog";
import { WardrobeDeleteDialog } from "./WardrobeDeleteDialog";
import { WardrobeEditDialog } from "./WardrobeEditDialog";
import { WardrobeListSection } from "./WardrobeListSection";
import { WardrobePreviewPanel } from "./WardrobePreviewPanel";
import { WardrobeUploadDialog } from "./WardrobeUploadDialog";

export default function TextureWardrobePage() {
	const { t } = useTranslation();
	const controller = useTextureWardrobePageController();
	const textureLibraryConfig = useFrontendConfigStore(
		(store) => store.textureLibrary,
	);

	usePageTitle(t("wardrobe.title"));

	const {
		activeTab,
		activeTexture,
		bindTexture,
		changeTagSearchMethod,
		clearBindDialogStateAfterClose,
		clearDeleteDialogStateAfterClose,
		clearEditDialogStateAfterClose,
		clearTagFilter,
		clearUploadDialogStateAfterClose,
		deleteDialogOpen,
		deleteTexture,
		deleteWardrobeTexture,
		dialogOpen,
		dragActive,
		dragTextureFile,
		dropTextureFile,
		editDialogOpen,
		editTagIds,
		editTagQuery,
		editTexture,
		file,
		filteredProfiles,
		hasMoreTags,
		leaveTextureDropZone,
		loadMoreTags,
		loading,
		model,
		openBindDialog,
		openDeleteDialog,
		openEditDialog,
		openUploadDialog,
		previewTexture,
		profileQuery,
		profiles,
		query,
		searchBusy,
		searchTags,
		selectedProfileId,
		selectedTags,
		selectTab,
		selectTextureFile,
		setDragActive,
		setDraftTagIds,
		setEditTagQuery,
		setTagFilterOpen,
		setTexturePageSize,
		setUploadDialogOpen,
		submitting,
		submitTextureLibrary,
		tagFilterOpen,
		tagLoading,
		tagSearchMethod,
		textureList,
		texturePageSize,
		textureTotal,
		textureType,
		toggleEditTag,
		updateWardrobeTexture,
		uploadDialogOpen,
		uploadName,
		uploadTexture,
		visibility,
		visibleTextures,
		withdrawTextureLibrary,
	} = controller;
	const { dispatch, draftTagIds, tags } = controller;

	return (
		<div className="mx-auto grid w-full max-w-[96rem] gap-4 px-4 py-5 sm:px-6 lg:px-7">
			<header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
				<div className="min-w-0">
					<h1 className="text-3xl font-semibold tracking-normal">
						{t("wardrobe.title")}
					</h1>
					<p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
						{t("wardrobe.description")}
					</p>
				</div>
				<p className="text-sm font-medium text-muted-foreground sm:pt-2">
					{t("wardrobe.heroAside")}
				</p>
			</header>

			<div className="grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(24rem,0.48fr)]">
				<WardrobeListSection
					activeTab={activeTab}
					draftTagIds={draftTagIds}
					hasMoreTags={hasMoreTags}
					loading={loading}
					onClearTagFilter={clearTagFilter}
					onChangeTagSearchMethod={changeTagSearchMethod}
					onLoadMoreTags={loadMoreTags}
					onOpenTagFilter={setTagFilterOpen}
					onPageSizeChange={(nextPageSize) => {
						// pageSize is a list dep, so the list resets itself.
						setTexturePageSize(nextPageSize);
					}}
					onQueryChange={(nextQuery) =>
						dispatch({ type: "query", value: nextQuery })
					}
					onReload={() => void textureList.reload()}
					onSearchQueryChange={searchTags}
					onSelectTab={selectTab}
					onSelectedTagIdsChange={(nextIds) => {
						setDraftTagIds(nextIds);
						dispatch({ type: "activeTexture", value: null });
					}}
					onTextureSelect={(texture) =>
						controller.dispatch({ type: "activeTexture", value: texture })
					}
					onUpload={() => openUploadDialog()}
					openUploadDialog={() => openUploadDialog()}
					pageSize={texturePageSize}
					query={query}
					searchBusy={searchBusy}
					selectedTags={selectedTags}
					tagFilterOpen={tagFilterOpen}
					tagLoading={tagLoading}
					tagSearchMethod={tagSearchMethod}
					tags={tags}
					textureList={{
						currentPage: textureList.currentPage,
						goNext: textureList.goNext,
						goPrevious: textureList.goPrevious,
						items: visibleTextures,
						nextPageDisabled: textureList.nextPageDisabled,
						prevPageDisabled: textureList.prevPageDisabled,
						total: textureTotal,
					}}
					previewTextureId={previewTexture?.id ?? null}
					uploading={submitting}
				/>

				<WardrobePreviewPanel
					libraryConfig={textureLibraryConfig}
					submitting={submitting}
					texture={previewTexture}
					total={textureTotal}
					onBind={() => {
						if (previewTexture) openBindDialog(previewTexture);
					}}
					onDelete={() => {
						if (previewTexture) openDeleteDialog(previewTexture);
					}}
					onEdit={() => {
						if (previewTexture) openEditDialog(previewTexture);
					}}
					onSubmitLibrary={() => {
						if (previewTexture) void submitTextureLibrary(previewTexture);
					}}
					onWithdrawLibrary={() => {
						if (previewTexture) void withdrawTextureLibrary(previewTexture);
					}}
				/>
			</div>

			<WardrobeUploadDialog
				clearStateAfterClose={clearUploadDialogStateAfterClose}
				dragActive={dragActive}
				file={file}
				model={model}
				onCancel={() => setUploadDialogOpen(false)}
				onDragEnter={dragTextureFile}
				onDragLeave={leaveTextureDropZone}
				onDrop={dropTextureFile}
				onFileChange={selectTextureFile}
				onModelChange={(nextModel) =>
					dispatch({ type: "model", value: nextModel })
				}
				onNameChange={(nextName) =>
					dispatch({ type: "uploadName", value: nextName })
				}
				onOpenChange={setUploadDialogOpen}
				onSubmit={uploadTexture}
				onTextureTypeChange={(nextType) => {
					dispatch({ type: "textureType", value: nextType });
					dispatch({ type: "file", value: null });
					dispatch({ type: "uploadName", value: "" });
					setDragActive(false);
				}}
				onVisibilityChange={(nextVisibility) =>
					dispatch({ type: "visibility", value: nextVisibility })
				}
				open={uploadDialogOpen}
				previewTexture={previewTexture}
				submitting={submitting}
				textureType={textureType}
				uploadName={uploadName}
				visibility={visibility}
			/>

			<WardrobeEditDialog
				clearStateAfterClose={clearEditDialogStateAfterClose}
				editDialogOpen={editDialogOpen}
				editTagIds={editTagIds}
				editTagQuery={editTagQuery}
				editTexture={editTexture}
				hasMoreTags={hasMoreTags}
				onLoadMoreTags={loadMoreTags}
				onOpenChange={(open) =>
					controller.dispatch({ type: "editDialogOpen", value: open })
				}
				onQueryChange={setEditTagQuery}
				onSearchTags={searchTags}
				onSubmit={updateWardrobeTexture}
				onToggleTag={toggleEditTag}
				submitting={submitting}
				tagLoading={tagLoading}
				tags={tags}
			/>

			<WardrobeBindDialog
				activeTexture={activeTexture}
				clearStateAfterClose={clearBindDialogStateAfterClose}
				dialogOpen={dialogOpen}
				filteredProfiles={filteredProfiles}
				onClose={(open) =>
					controller.dispatch({ type: "dialogOpen", value: open })
				}
				onProfileQueryChange={(nextQuery) =>
					controller.dispatch({ type: "profileQuery", value: nextQuery })
				}
				onProfileSelect={(profileId) =>
					controller.dispatch({ type: "selectedProfileId", value: profileId })
				}
				onSubmitBind={() => void bindTexture()}
				profiles={profiles}
				profileQuery={profileQuery}
				selectedProfileId={selectedProfileId}
				submitting={submitting}
			/>

			<WardrobeDeleteDialog
				clearStateAfterClose={clearDeleteDialogStateAfterClose}
				deleteDialogOpen={deleteDialogOpen}
				deleteTexture={deleteTexture}
				onClose={(open) =>
					controller.dispatch({ type: "deleteDialogOpen", value: open })
				}
				onConfirmDelete={() => void deleteWardrobeTexture()}
				submitting={submitting}
			/>
		</div>
	);
}

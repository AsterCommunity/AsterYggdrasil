import {
	type FormEvent,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useCursorList } from "@/hooks/useCursorList";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useTextureTagPager } from "@/hooks/useTextureTagPager";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { textureTagStore } from "@/stores/textureTagStore";
import type {
	DateTimeIdCursor,
	MinecraftTextureType,
	PublicTextureLibraryTextureMetadata,
	TextureTagSearchMethod,
} from "@/types/api";

const TAG_FILTER_APPLY_DEBOUNCE_MS = 240;

function sameNumberArray(left: number[], right: number[]) {
	return (
		left.length === right.length &&
		left.every((value, index) => value === right[index])
	);
}

export function usePublicTextureLibraryController(defaultPageSize: number) {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const textureLibraryEnabled = useFrontendConfigStore(
		(state) => state.textureLibrary.enabled,
	);
	const [textureType, setTextureType] = useState<MinecraftTextureType | "all">(
		"all",
	);
	const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
	const [draftTagIds, setDraftTagIds] = useState<number[]>([]);
	const [tagFilterOpen, setTagFilterOpen] = useState(false);
	const [tagSearchMethod, setTagSearchMethod] =
		useState<TextureTagSearchMethod>("all");
	const [query, setQuery] = useState("");
	const [appliedQuery, setAppliedQuery] = useState("");
	const [pageSize, setPageSize] = useState<number>(defaultPageSize);
	const serverName = branding.title || t("home.titleFallback");

	const tagPager = useTextureTagPager({
		loadPage: (params) =>
			textureTagStore.getPublicTextureTagPage(params, () =>
				yggdrasilService.listPublicTextureLibraryTags(params),
			),
		onError: (error) => toast.error(formatUnknownError(error)),
		retainedTagIds: draftTagIds,
	});
	const {
		ensureLoaded: ensureTagsLoaded,
		hasMore: hasMoreTags,
		loadMore: loadMoreTags,
		loading: tagLoading,
		search: searchTags,
		tags,
	} = tagPager;

	const loadTexturePage = useCallback(
		async ({ cursor }: { cursor: DateTimeIdCursor | null }) => {
			const page = await yggdrasilService.listPublicTextureLibraryTextures({
				limit: pageSize,
				after_updated_at: cursor?.value,
				after_id: cursor?.id,
				keyword: appliedQuery.trim() || undefined,
				texture_type: textureType === "all" ? undefined : textureType,
				tag_ids: selectedTagIds.length > 0 ? selectedTagIds : undefined,
				tag_search_method:
					selectedTagIds.length > 0 ? tagSearchMethod : undefined,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[appliedQuery, pageSize, selectedTagIds, tagSearchMethod, textureType],
	);
	const list = useCursorList<
		PublicTextureLibraryTextureMetadata,
		DateTimeIdCursor
	>({
		deps: [
			pageSize,
			appliedQuery,
			textureType,
			selectedTagIds,
			tagSearchMethod,
		],
		enabled: textureLibraryEnabled,
		loadPage: loadTexturePage,
		onError: (error) => toast.error(formatUnknownError(error)),
	});
	const { loading, total } = list;
	const textures = list.items;
	const resetCursor = list.reset;

	// Draft tag selections commit after a short debounce so quick toggles
	// collapse into one request.
	const debouncedDraftTagIds = useDebouncedValue(
		draftTagIds,
		TAG_FILTER_APPLY_DEBOUNCE_MS,
	);
	useEffect(() => {
		setSelectedTagIds((current) =>
			sameNumberArray(current, debouncedDraftTagIds)
				? current
				: debouncedDraftTagIds,
		);
	}, [debouncedDraftTagIds]);

	useEffect(() => {
		if (textureLibraryEnabled && tagFilterOpen) {
			ensureTagsLoaded();
		}
	}, [ensureTagsLoaded, tagFilterOpen, textureLibraryEnabled]);

	const selectedTags = useMemo(
		() => tags.filter((tag) => draftTagIds.includes(tag.id)),
		[tags, draftTagIds],
	);

	function submitSearch(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		// appliedQuery is a list dep, so the list resets itself.
		setAppliedQuery(query);
	}

	function clearTagFilter() {
		setDraftTagIds([]);
	}

	function changeTagSearchMethod(nextMethod: TextureTagSearchMethod) {
		// The search method is a list dep, so the list resets itself.
		setTagSearchMethod(nextMethod);
	}

	const currentPage = list.currentPage;
	const totalPages = Math.max(1, Math.ceil(total / pageSize));

	return {
		branding,
		draftTagIds,
		changePageSize: (nextPageSize: number) => {
			setPageSize(nextPageSize);
			resetCursor();
		},
		changeTagSearchMethod,
		clearTagFilter,
		currentPage,
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
		submitSearch,
		tagFilterOpen,
		tagLoading,
		tagSearchMethod,
		textureLibraryEnabled,
		textures,
		total,
		totalPages,
		textureType,
		setTextureType,
		hasMoreTags,
		loadMoreTags,
		tags,
	};
}

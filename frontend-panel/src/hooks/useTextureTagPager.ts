import { useCallback, useEffect, useRef, useState } from "react";
import { TEXTURE_TAG_PAGE_SIZE } from "@/services/yggdrasilService";
import type { MinecraftTextureTagInfo } from "@/types/api";

type TextureTagPage = {
	items: MinecraftTextureTagInfo[];
	total: number;
};

type TextureTagPageLoader = (params: {
	keyword?: string;
	limit: number;
	offset: number;
}) => Promise<TextureTagPage>;

export function useTextureTagPager({
	loadPage,
	onError,
	retainedTagIds,
}: {
	loadPage: TextureTagPageLoader;
	onError: (error: unknown) => void;
	retainedTagIds: number[];
}) {
	const [tags, setTags] = useState<MinecraftTextureTagInfo[]>([]);
	const [total, setTotal] = useState(0);
	const [offset, setOffset] = useState(0);
	const [keyword, setKeyword] = useState("");
	const [loading, setLoading] = useState(false);
	const loadPageRef = useRef(loadPage);
	const onErrorRef = useRef(onError);
	const retainedTagIdsRef = useRef(retainedTagIds);
	const hasMore = offset < total;

	useEffect(() => {
		loadPageRef.current = loadPage;
	}, [loadPage]);

	useEffect(() => {
		onErrorRef.current = onError;
	}, [onError]);

	useEffect(() => {
		retainedTagIdsRef.current = retainedTagIds;
	}, [retainedTagIds]);

	const load = useCallback(
		async (
			params: { keyword?: string; offset?: number; append?: boolean } = {},
		) => {
			const nextKeyword = params.keyword ?? keyword;
			const nextOffset = params.offset ?? 0;
			setLoading(true);
			try {
				const page = await loadPageRef.current({
					limit: TEXTURE_TAG_PAGE_SIZE,
					offset: nextOffset,
					keyword: nextKeyword || undefined,
				});
				setTags((current) => {
					if (params.append) {
						return mergeTextureTags(current, page.items);
					}
					const retainedTagIdSet = new Set(retainedTagIdsRef.current);
					const retained = current.filter((tag) =>
						retainedTagIdSet.has(tag.id),
					);
					return mergeTextureTags(retained, page.items);
				});
				setTotal(page.total);
				setOffset(nextOffset + page.items.length);
			} catch (error) {
				onErrorRef.current(error);
			} finally {
				setLoading(false);
			}
		},
		[keyword],
	);

	const ensureLoaded = useCallback(() => {
		if (tags.length > 0 || loading) return;
		void load({ keyword, offset: 0 });
	}, [keyword, load, loading, tags.length]);

	const search = useCallback(
		(nextKeyword: string) => {
			if (nextKeyword === keyword) return;
			setKeyword(nextKeyword);
			setOffset(0);
			void load({ keyword: nextKeyword, offset: 0 });
		},
		[keyword, load],
	);

	const loadMore = useCallback(() => {
		if (loading || !hasMore) return;
		void load({ keyword, offset, append: true });
	}, [hasMore, keyword, load, loading, offset]);

	const addTags = useCallback((nextTags: MinecraftTextureTagInfo[]) => {
		setTags((current) => mergeTextureTags(current, nextTags));
	}, []);

	return {
		addTags,
		ensureLoaded,
		hasMore,
		keyword,
		load,
		loadMore,
		loading,
		search,
		tags,
	};
}

function mergeTextureTags(
	current: MinecraftTextureTagInfo[],
	next: MinecraftTextureTagInfo[],
) {
	const tagsById = new Map<number, MinecraftTextureTagInfo>();
	for (const tag of current) {
		tagsById.set(tag.id, tag);
	}
	for (const tag of next) {
		tagsById.set(tag.id, tag);
	}
	return Array.from(tagsById.values());
}

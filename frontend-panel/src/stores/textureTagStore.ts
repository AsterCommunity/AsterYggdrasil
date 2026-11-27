import { TtlPromiseCache } from "@/lib/ttlPromiseCache";
import type {
	MinecraftTextureTagPage,
	MinecraftTextureTagQuery,
	PublicTextureLibraryTagPage,
	PublicTextureLibraryTagQuery,
} from "@/types/api";

export const TEXTURE_TAG_CACHE_TTL_MS = 60_000;

const textureTagCaches = {
	textureTagPageCache: new TtlPromiseCache<MinecraftTextureTagPage>(
		TEXTURE_TAG_CACHE_TTL_MS,
	),
	publicTextureTagPageCache: new TtlPromiseCache<PublicTextureLibraryTagPage>(
		TEXTURE_TAG_CACHE_TTL_MS,
	),
};

function tagPageCacheKey(params: {
	after_id?: number | null;
	after_name?: string | null;
	after_sort_order?: number | null;
	keyword?: string | null;
	limit?: number | null;
}) {
	return JSON.stringify({
		after_id: params.after_id ?? null,
		after_name: params.after_name ?? null,
		after_sort_order: params.after_sort_order ?? null,
		keyword: params.keyword?.trim() || "",
		limit: params.limit ?? null,
	});
}

/**
 * Cache owner for texture-tag lookups shared across pickers (wardrobe and
 * public library tag filters). The yggdrasil service stays a pure adapter;
 * this module is the single explicit home for tag caching: normalized query
 * key, 60s TTL, in-flight deduplication and explicit force refresh.
 */
export const textureTagStore = {
	getTextureTagPage(
		params: MinecraftTextureTagQuery,
		loader: () => Promise<MinecraftTextureTagPage>,
		options: { force?: boolean } = {},
	) {
		return textureTagCaches.textureTagPageCache.run(
			tagPageCacheKey(params),
			loader,
			Boolean(options.force),
		);
	},
	getPublicTextureTagPage(
		params: PublicTextureLibraryTagQuery,
		loader: () => Promise<PublicTextureLibraryTagPage>,
		options: { force?: boolean } = {},
	) {
		return textureTagCaches.publicTextureTagPageCache.run(
			tagPageCacheKey(params),
			loader,
			Boolean(options.force),
		);
	},
	clearTextureTagPages() {
		textureTagCaches.textureTagPageCache.clear();
	},
	clearPublicTextureTagPages() {
		textureTagCaches.publicTextureTagPageCache.clear();
	},
};

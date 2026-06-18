import { config } from "@/config/app";
import { withQuery } from "@/lib/query";
import { api } from "@/services/http";
import type {
	CopyPublicTextureRequest,
	CreateMinecraftProfileRequest,
	MinecraftTextureMetadata,
	MinecraftTextureModel,
	MinecraftTextureTagList,
	MinecraftTextureTagPage,
	MinecraftTextureTagQuery,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	MinecraftWardrobeTextureMetadata,
	MinecraftWardrobeTexturePage,
	MinecraftWardrobeTextureQuery,
	OperationJsonResponse,
	OperationPath,
	OperationQuery,
	OperationRequestBody,
	PublicTextureLibraryPage,
	PublicTextureLibraryQuery,
	PublicTextureLibraryTagPage,
	PublicTextureLibraryTagQuery,
	PublicTextureLibraryTextureMetadata,
	PublicYggdrasilConfig,
	RenameMinecraftProfileRequest,
	ReplaceWardrobeTextureTagsRequest,
	UpdateWardrobeTextureRequest,
	YggdrasilErrorBody,
	YggdrasilMetadata,
	YggdrasilProfile,
	YggdrasilProfilePage,
	YggdrasilProfileQuery,
} from "@/types/api";

type YggdrasilProfileByUuidQuery = OperationQuery<"yggdrasil_profile_by_uuid">;
type YggdrasilProfileByUuidPath = OperationPath<"yggdrasil_profile_by_uuid">;
type YggdrasilTexturePath = OperationPath<"yggdrasil_upload_texture">;
type YggdrasilProfileByUuidResponse =
	OperationJsonResponse<"yggdrasil_profile_by_uuid">;

export type { YggdrasilMetadata };

export const TEXTURE_TAG_PAGE_SIZE = 30;
const TEXTURE_TAG_CACHE_TTL_MS = 60_000;

const textureTagPageCache = new Map<
	string,
	{
		expiresAt: number;
		promise: Promise<MinecraftTextureTagPage>;
	}
>();
const publicTextureTagPageCache = new Map<
	string,
	{
		expiresAt: number;
		promise: Promise<PublicTextureLibraryTagPage>;
	}
>();
let textureTagListCache: {
	expiresAt: number;
	promise: Promise<MinecraftTextureTagList>;
} | null = null;

const profileTexturesRequests = new Map<
	string,
	Promise<MinecraftTextureMetadata[]>
>();

export class YggdrasilProtocolError extends Error {
	status: number;
	error: string;
	cause?: string | null;

	constructor(status: number, body: YggdrasilErrorBody) {
		super(body.errorMessage);
		this.name = "YggdrasilProtocolError";
		this.status = status;
		this.error = body.error;
		this.cause = body.cause;
	}
}

function normalizeRootUrl(base = config.rootBaseUrl || "/") {
	if (typeof window === "undefined") {
		return base;
	}
	return new URL(base, window.location.origin).toString();
}

export function yggdrasilApiRoot(
	yggdrasilConfig?: Pick<PublicYggdrasilConfig, "public_base_urls"> | null,
) {
	const configuredRoot = yggdrasilConfig?.public_base_urls
		.find((url) => url.trim().length > 0)
		?.trim();
	return normalizeRootUrl(configuredRoot);
}

export function yggdrasilAddServerUri(apiRoot = yggdrasilApiRoot()) {
	return `authlib-injector:yggdrasil-server:${encodeURIComponent(apiRoot)}`;
}

export function yggdrasilPrefetchedMetadata(metadata: YggdrasilMetadata) {
	const bytes = new TextEncoder().encode(JSON.stringify(metadata));
	const binary = Array.from(bytes, (byte) => String.fromCharCode(byte)).join(
		"",
	);
	return btoa(binary);
}

function tagPageCacheKey(params: {
	limit?: number;
	offset?: number;
	keyword?: string;
}) {
	return JSON.stringify({
		keyword: params.keyword?.trim() || "",
		limit: params.limit ?? TEXTURE_TAG_PAGE_SIZE,
		offset: params.offset ?? 0,
	});
}

function normalizeTagKeyword(keyword: string | null | undefined) {
	const trimmed = keyword?.trim();
	return trimmed ? trimmed : undefined;
}

function cacheTextureTagPage<Page extends MinecraftTextureTagPage>(
	cache: Map<
		string,
		{
			expiresAt: number;
			promise: Promise<Page>;
		}
	>,
	key: string,
	promiseFactory: () => Promise<Page>,
	force: boolean,
) {
	const now = Date.now();
	if (!force) {
		const cached = cache.get(key);
		if (cached && cached.expiresAt > now) {
			return cached.promise;
		}
	}
	const promise = promiseFactory().catch((error) => {
		if (cache.get(key)?.promise === promise) {
			cache.delete(key);
		}
		throw error;
	});
	cache.set(key, {
		expiresAt: now + TEXTURE_TAG_CACHE_TTL_MS,
		promise,
	});
	return promise;
}

async function fetchAllTextureLibraryTags(): Promise<MinecraftTextureTagList> {
	const items: MinecraftTextureTagList = [];
	let offset = 0;
	let total = Number.POSITIVE_INFINITY;

	while (offset < total) {
		const page = await yggdrasilService.listTextureLibraryTagsPage({
			limit: TEXTURE_TAG_PAGE_SIZE,
			offset,
		});
		items.push(...page.items);
		total = page.total;
		if (page.items.length === 0) break;
		offset += page.items.length;
	}

	return items;
}

function cachedTextureLibraryTags(force = false) {
	const now = Date.now();
	if (!force && textureTagListCache && textureTagListCache.expiresAt > now) {
		return textureTagListCache.promise;
	}

	const promise = fetchAllTextureLibraryTags().catch((error) => {
		if (textureTagListCache?.promise === promise) {
			textureTagListCache = null;
		}
		throw error;
	});
	textureTagListCache = {
		expiresAt: now + TEXTURE_TAG_CACHE_TTL_MS,
		promise,
	};
	return promise;
}

function listProfileTextures(
	uuid: YggdrasilProfileByUuidPath["uuid"],
): Promise<MinecraftTextureMetadata[]> {
	const pending = profileTexturesRequests.get(uuid);
	if (pending) return pending;

	const promise = api
		.get<MinecraftTextureMetadata[]>(`/profiles/minecraft/${uuid}/textures`)
		.finally(() => {
			if (profileTexturesRequests.get(uuid) === promise) {
				profileTexturesRequests.delete(uuid);
			}
		});
	profileTexturesRequests.set(uuid, promise);
	return promise;
}

export const yggdrasilService = {
	async metadata(signal?: AbortSignal) {
		const response = await api.rootClient.get<YggdrasilMetadata>("/", {
			signal,
		});
		return response.data;
	},
	listProfiles: (params: YggdrasilProfileQuery = {}) =>
		api.get<YggdrasilProfilePage>(withQuery("/profiles/minecraft", params)),
	listProfileItems: async (params: YggdrasilProfileQuery = {}) =>
		(await yggdrasilService.listProfiles(params)).items,
	createProfile: (data: CreateMinecraftProfileRequest) =>
		api.post<
			YggdrasilProfile,
			OperationRequestBody<"create_current_user_minecraft_profile">
		>("/profiles/minecraft", data),
	renameProfile: (
		uuid: YggdrasilProfileByUuidPath["uuid"],
		data: RenameMinecraftProfileRequest,
	) =>
		api.put<
			YggdrasilProfile,
			OperationRequestBody<"rename_current_user_minecraft_profile">
		>(`/profiles/minecraft/${uuid}/name`, data),
	deleteProfile: (uuid: YggdrasilProfileByUuidPath["uuid"]) =>
		api.delete<void>(`/profiles/minecraft/${uuid}`),
	listProfileTextures,
	async listProfileSkinTextureUrls(
		uuids: YggdrasilProfileByUuidPath["uuid"][],
	) {
		const pairs = await Promise.all(
			uuids.map(async (uuid) => {
				const textures = await yggdrasilService.listProfileTextures(uuid);
				const skin = textures.find(
					(texture) =>
						texture.texture_type === "skin" && texture.source === "bound",
				);
				return [uuid, skin?.url ?? null] as const;
			}),
		);
		return Object.fromEntries(pairs) as Record<string, string | null>;
	},
	listWardrobeTextures: (params: MinecraftWardrobeTextureQuery = {}) =>
		api.get<MinecraftWardrobeTexturePage>(
			withQuery("/wardrobe/textures", params),
		),
	listTextureLibraryTags: (options: { force?: boolean } = {}) =>
		cachedTextureLibraryTags(Boolean(options.force)),
	listTextureLibraryTagsPage: (
		params: MinecraftTextureTagQuery = {},
		options: { force?: boolean } = {},
	) => {
		const keyword = normalizeTagKeyword(params.keyword);
		const nextParams = {
			limit: params.limit ?? TEXTURE_TAG_PAGE_SIZE,
			offset: params.offset ?? 0,
			keyword,
		};
		const key = tagPageCacheKey(nextParams);
		return cacheTextureTagPage(
			textureTagPageCache,
			key,
			() =>
				api.get<MinecraftTextureTagPage>(
					withQuery("/wardrobe/tags", nextParams),
				),
			Boolean(options.force),
		);
	},
	listWardrobeTextureItems: async (
		params: MinecraftWardrobeTextureQuery = {},
	) => (await yggdrasilService.listWardrobeTextures(params)).items,
	listPublicTextureLibraryTags: (
		params: PublicTextureLibraryTagQuery = {},
		options: { force?: boolean } = {},
	) => {
		const keyword = normalizeTagKeyword(params.keyword);
		const nextParams = {
			limit: params.limit ?? TEXTURE_TAG_PAGE_SIZE,
			offset: params.offset ?? 0,
			keyword,
		};
		const key = tagPageCacheKey(nextParams);
		return cacheTextureTagPage(
			publicTextureTagPageCache,
			key,
			() =>
				api.get<PublicTextureLibraryTagPage>(
					withQuery("/texture-library/tags", nextParams),
				),
			Boolean(options.force),
		);
	},
	listPublicTextureLibraryTextures: (params: PublicTextureLibraryQuery = {}) =>
		api.get<PublicTextureLibraryPage>(
			withQuery("/texture-library/textures", params),
		),
	getPublicTextureLibraryTexture: (textureId: number) =>
		api.get<PublicTextureLibraryTextureMetadata>(
			`/texture-library/textures/${textureId}`,
		),
	copyPublicTextureToWardrobe: (
		textureId: number,
		data: CopyPublicTextureRequest = {},
	) =>
		api.post<MinecraftWardrobeTextureMetadata, CopyPublicTextureRequest>(
			`/texture-library/textures/${textureId}/copy`,
			data,
		),
	deleteWardrobeTexture: (textureId: number) =>
		api.delete<void>(`/wardrobe/textures/${textureId}`),
	async uploadWardrobeTexture(params: {
		textureType: MinecraftTextureType;
		file: File;
		model?: MinecraftTextureModel;
		name?: string;
		visibility?: MinecraftTextureVisibility;
	}) {
		const form = new FormData();
		if (params.textureType === "skin") {
			form.append("model", params.model === "slim" ? "slim" : "");
		}
		const name = params.name?.trim();
		if (name) {
			form.append("name", name);
		}
		form.append("visibility", params.visibility ?? "private");
		form.append("file", params.file);
		return api.post<MinecraftWardrobeTextureMetadata, FormData>(
			`/wardrobe/textures/${params.textureType}`,
			form,
		);
	},
	updateWardrobeTexture: (
		textureId: number,
		data: UpdateWardrobeTextureRequest,
	) =>
		api.patch<MinecraftWardrobeTextureMetadata, UpdateWardrobeTextureRequest>(
			`/wardrobe/textures/${textureId}`,
			data,
		),
	replaceWardrobeTextureTags: (
		textureId: number,
		data: ReplaceWardrobeTextureTagsRequest,
	) =>
		api.put<
			MinecraftWardrobeTextureMetadata,
			ReplaceWardrobeTextureTagsRequest
		>(`/wardrobe/textures/${textureId}/tags`, data),
	bindProfileTexture: (params: {
		uuid: YggdrasilProfileByUuidPath["uuid"];
		textureType: MinecraftTextureType;
		textureId: number;
	}) =>
		api.put<
			MinecraftTextureMetadata,
			OperationRequestBody<"bind_current_user_minecraft_profile_texture">
		>(`/profiles/minecraft/${params.uuid}/textures/${params.textureType}`, {
			texture_id: params.textureId,
		}),
	unbindProfileTexture: (params: {
		uuid: YggdrasilProfileByUuidPath["uuid"];
		textureType: MinecraftTextureType;
	}) =>
		api.delete<void>(
			`/profiles/minecraft/${params.uuid}/textures/${params.textureType}`,
		),
	async profileByUuid(
		uuid: YggdrasilProfileByUuidPath["uuid"],
		unsigned: YggdrasilProfileByUuidQuery["unsigned"] = false,
		signal?: AbortSignal,
	) {
		const response = await api.rootClient.get<YggdrasilProfileByUuidResponse>(
			withQuery(`/sessionserver/session/minecraft/profile/${uuid}`, {
				unsigned,
			}),
			{ signal },
		);
		return response.data;
	},
	async uploadTexture(params: {
		accessToken: string;
		uuid: YggdrasilTexturePath["uuid"];
		textureType: MinecraftTextureType;
		file: File;
		model?: MinecraftTextureModel;
	}) {
		const form = new FormData();
		if (params.textureType === "skin") {
			form.append("model", params.model === "slim" ? "slim" : "");
		}
		form.append("file", params.file);
		await requestYggdrasilNoContent(
			"put",
			`/api/user/profile/${params.uuid}/${params.textureType}`,
			params.accessToken,
			form,
		);
	},
	async deleteTexture(params: {
		accessToken: string;
		uuid: YggdrasilTexturePath["uuid"];
		textureType: MinecraftTextureType;
	}) {
		await requestYggdrasilNoContent(
			"delete",
			`/api/user/profile/${params.uuid}/${params.textureType}`,
			params.accessToken,
		);
	},
};

async function requestYggdrasilNoContent(
	method: "put" | "delete",
	url: string,
	accessToken: string,
	body?: FormData,
) {
	try {
		await api.rootClient.request({
			method,
			url,
			data: body,
			headers: {
				Authorization: `Bearer ${accessToken}`,
			},
		});
	} catch (error) {
		if (
			typeof error === "object" &&
			error !== null &&
			"response" in error &&
			typeof error.response === "object" &&
			error.response !== null &&
			"status" in error.response &&
			"data" in error.response
		) {
			const status = Number(error.response.status);
			const data = error.response.data;
			if (isYggdrasilErrorBody(data)) {
				throw new YggdrasilProtocolError(status, data);
			}
		}
		throw error;
	}
}

function isYggdrasilErrorBody(value: unknown): value is YggdrasilErrorBody {
	if (typeof value !== "object" || value === null) return false;
	return (
		"error" in value &&
		"errorMessage" in value &&
		typeof value.error === "string" &&
		typeof value.errorMessage === "string"
	);
}

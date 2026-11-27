import { config } from "@/config/app";
import { withQuery } from "@/lib/query";
import { api } from "@/services/http";
import type {
	CopyPublicTextureRequest,
	CreateMinecraftProfileRequest,
	CreateTextureReportRequest,
	MinecraftTextureMetadata,
	MinecraftTextureModel,
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

function normalizeTagKeyword(keyword: string | null | undefined) {
	const trimmed = keyword?.trim();
	return trimmed ? trimmed : undefined;
}

function listProfileTextures(
	uuid: YggdrasilProfileByUuidPath["uuid"],
): Promise<MinecraftTextureMetadata[]> {
	return api.get<MinecraftTextureMetadata[]>(
		`/profiles/minecraft/${uuid}/textures`,
	);
}

export const yggdrasilService = {
	async metadata(signal?: AbortSignal) {
		const response = await api.rootClient.get<YggdrasilMetadata>("/", {
			signal,
		});
		return response.data;
	},
	listProfiles: (params: YggdrasilProfileQuery = {}) =>
		api.get<YggdrasilProfilePage>(
			withQuery("/profiles/minecraft", {
				limit: params.limit,
				after_id: params.after_id,
				query: params.query,
			}),
		),
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
			withQuery("/wardrobe/textures", {
				limit: params.limit,
				after_updated_at: params.after_updated_at,
				after_id: params.after_id,
				keyword: params.keyword,
				texture_type: params.texture_type,
				tag_ids: params.tag_ids,
				tag_search_method: params.tag_search_method,
			}),
		),
	listTextureLibraryTagsPage: (params: MinecraftTextureTagQuery = {}) => {
		const keyword = normalizeTagKeyword(params.keyword);
		return api.get<MinecraftTextureTagPage>(
			withQuery("/wardrobe/tags", {
				limit: params.limit ?? TEXTURE_TAG_PAGE_SIZE,
				keyword: keyword ?? undefined,
				after_sort_order: params.after_sort_order ?? undefined,
				after_name: params.after_name ?? undefined,
				after_id: params.after_id ?? undefined,
			}),
		);
	},
	listWardrobeTextureItems: async (
		params: MinecraftWardrobeTextureQuery = {},
	) => (await yggdrasilService.listWardrobeTextures(params)).items,
	listPublicTextureLibraryTags: (params: PublicTextureLibraryTagQuery = {}) => {
		const keyword = normalizeTagKeyword(params.keyword);
		return api.get<PublicTextureLibraryTagPage>(
			withQuery("/texture-library/tags", {
				limit: params.limit ?? TEXTURE_TAG_PAGE_SIZE,
				keyword: keyword ?? undefined,
				after_sort_order: params.after_sort_order ?? undefined,
				after_name: params.after_name ?? undefined,
				after_id: params.after_id ?? undefined,
			}),
		);
	},
	listPublicTextureLibraryTextures: (params: PublicTextureLibraryQuery = {}) =>
		api.get<PublicTextureLibraryPage>(
			withQuery("/texture-library/textures", {
				limit: params.limit,
				after_updated_at: params.after_updated_at,
				after_id: params.after_id,
				keyword: params.keyword,
				texture_type: params.texture_type,
				tag_ids: params.tag_ids,
				tag_search_method: params.tag_search_method,
			}),
		),
	getPublicTextureLibraryTexture: (textureId: number) =>
		api.get<PublicTextureLibraryTextureMetadata>(
			`/texture-library/textures/${textureId}`,
		),
	submitTextureLibraryReview: (textureId: number) =>
		api.post<MinecraftWardrobeTextureMetadata>(
			`/wardrobe/textures/${textureId}/library-submission`,
		),
	withdrawTextureLibrarySubmission: (textureId: number) =>
		api.delete<MinecraftWardrobeTextureMetadata>(
			`/wardrobe/textures/${textureId}/library-submission`,
		),
	copyPublicTextureToWardrobe: (
		textureId: number,
		data: CopyPublicTextureRequest = {},
	) =>
		api.post<MinecraftWardrobeTextureMetadata, CopyPublicTextureRequest>(
			`/texture-library/textures/${textureId}/copy`,
			data,
		),
	createTextureReport: (textureId: number, data: CreateTextureReportRequest) =>
		api.post<
			OperationJsonResponse<"create_public_texture_library_texture_report">,
			CreateTextureReportRequest
		>(`/texture-library/textures/${textureId}/reports`, data),
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

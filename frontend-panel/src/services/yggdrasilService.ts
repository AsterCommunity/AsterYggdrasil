import { config } from "@/config/app";
import { withQuery } from "@/lib/query";
import { api } from "@/services/http";
import type {
	CreateMinecraftProfileRequest,
	MinecraftTextureMetadata,
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	MinecraftWardrobeTextureMetadata,
	MinecraftWardrobeTexturePage,
	MinecraftWardrobeTextureQuery,
	OperationJsonResponse,
	OperationPath,
	OperationQuery,
	OperationRequestBody,
	PublicYggdrasilConfig,
	RenameMinecraftProfileRequest,
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
	listProfileTextures: (uuid: YggdrasilProfileByUuidPath["uuid"]) =>
		api.get<MinecraftTextureMetadata[]>(`/profiles/minecraft/${uuid}/textures`),
	listWardrobeTextures: (params: MinecraftWardrobeTextureQuery = {}) =>
		api.get<MinecraftWardrobeTexturePage>(
			withQuery("/wardrobe/textures", params),
		),
	listWardrobeTextureItems: async (
		params: MinecraftWardrobeTextureQuery = {},
	) => (await yggdrasilService.listWardrobeTextures(params)).items,
	deleteWardrobeTexture: (textureId: number) =>
		api.delete<void>(`/wardrobe/textures/${textureId}`),
	async uploadWardrobeTexture(params: {
		textureType: MinecraftTextureType;
		file: File;
		model?: MinecraftTextureModel;
		visibility?: MinecraftTextureVisibility;
	}) {
		const form = new FormData();
		if (params.textureType === "skin") {
			form.append("model", params.model === "slim" ? "slim" : "");
		}
		form.append("visibility", params.visibility ?? "private");
		form.append("file", params.file);
		return api.post<MinecraftWardrobeTextureMetadata, FormData>(
			`/wardrobe/textures/${params.textureType}`,
			form,
		);
	},
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

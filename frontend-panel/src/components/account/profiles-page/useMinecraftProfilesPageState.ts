import { useReducer } from "react";
import type {
	MinecraftTextureMetadata,
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
} from "@/types/api";

export type MinecraftProfilesPageState = {
	deleteDialogOpen: boolean;
	deleteProfileDialogOpen: boolean;
	deletingProfile: boolean;
	dragActive: boolean;
	file: File | null;
	loading: boolean;
	model: MinecraftTextureModel;
	profilePageSize: number;
	profileName: string;
	profileSkinUrls: Record<string, string | null>;
	query: string;
	renameDialogOpen: boolean;
	renameName: string;
	renaming: boolean;
	selectedUuid: string;
	textures: MinecraftTextureMetadata[];
	texturesLoading: boolean;
	textureDialogOpen: boolean;
	textureManageDialogOpen: boolean;
	textureType: MinecraftTextureType;
	uploadTextureType: MinecraftTextureType;
	visibility: MinecraftTextureVisibility;
};

export type MinecraftProfilesPageAction =
	| { type: "deleteDialogOpen"; value: boolean }
	| { type: "deleteProfileDialogOpen"; value: boolean }
	| { type: "deletingProfile"; value: boolean }
	| { type: "dragActive"; value: boolean }
	| { type: "file"; value: File | null }
	| { type: "loading"; value: boolean }
	| { type: "model"; value: MinecraftTextureModel }
	| { type: "profilePageSize"; value: number }
	| { type: "profileName"; value: string }
	| { type: "profileSkinUrls"; value: Record<string, string | null> }
	| { type: "query"; value: string }
	| { type: "renameDialogOpen"; value: boolean }
	| { type: "renameName"; value: string }
	| { type: "renaming"; value: boolean }
	| { type: "selectedUuid"; value: string }
	| { type: "textures"; value: MinecraftTextureMetadata[] }
	| { type: "texturesLoading"; value: boolean }
	| { type: "textureDialogOpen"; value: boolean }
	| { type: "textureManageDialogOpen"; value: boolean }
	| { type: "textureType"; value: MinecraftTextureType }
	| { type: "uploadTextureType"; value: MinecraftTextureType }
	| { type: "visibility"; value: MinecraftTextureVisibility };

const initialState: MinecraftProfilesPageState = {
	deleteDialogOpen: false,
	deleteProfileDialogOpen: false,
	deletingProfile: false,
	dragActive: false,
	file: null,
	loading: false,
	model: "default",
	profilePageSize: 5,
	profileName: "",
	profileSkinUrls: {},
	query: "",
	renameDialogOpen: false,
	renameName: "",
	renaming: false,
	selectedUuid: "",
	textures: [],
	texturesLoading: false,
	textureDialogOpen: false,
	textureManageDialogOpen: false,
	textureType: "skin",
	uploadTextureType: "skin",
	visibility: "private",
};

function reducer(
	state: MinecraftProfilesPageState,
	action: MinecraftProfilesPageAction,
): MinecraftProfilesPageState {
	switch (action.type) {
		case "file":
		case "deleteDialogOpen":
		case "deleteProfileDialogOpen":
		case "deletingProfile":
		case "dragActive":
		case "loading":
		case "model":
		case "profilePageSize":
		case "profileName":
		case "profileSkinUrls":
		case "query":
		case "renameDialogOpen":
		case "renameName":
		case "renaming":
		case "selectedUuid":
		case "textures":
		case "texturesLoading":
		case "textureDialogOpen":
		case "textureManageDialogOpen":
		case "textureType":
		case "uploadTextureType":
		case "visibility":
			return { ...state, [action.type]: action.value };
	}
}

export function useMinecraftProfilesPageState() {
	return useReducer(reducer, initialState);
}

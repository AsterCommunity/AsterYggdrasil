import { useReducer } from "react";
import type {
	MinecraftTextureMetadata,
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	YggdrasilProfile,
} from "@/types/api";

export type MinecraftProfilesPageState = {
	file: File | null;
	loading: boolean;
	model: MinecraftTextureModel;
	profileName: string;
	profileTotal: number;
	profiles: YggdrasilProfile[];
	query: string;
	selectedUuid: string;
	textures: MinecraftTextureMetadata[];
	texturesLoading: boolean;
	textureType: MinecraftTextureType;
	visibility: MinecraftTextureVisibility;
};

export type MinecraftProfilesPageAction =
	| { type: "file"; value: File | null }
	| { type: "loading"; value: boolean }
	| { type: "model"; value: MinecraftTextureModel }
	| { type: "profileName"; value: string }
	| { type: "profilePage"; value: { items: YggdrasilProfile[]; total: number } }
	| { type: "profiles"; value: YggdrasilProfile[] }
	| { type: "query"; value: string }
	| { type: "selectedUuid"; value: string }
	| { type: "textures"; value: MinecraftTextureMetadata[] }
	| { type: "texturesLoading"; value: boolean }
	| { type: "textureType"; value: MinecraftTextureType }
	| { type: "visibility"; value: MinecraftTextureVisibility };

const initialState: MinecraftProfilesPageState = {
	file: null,
	loading: false,
	model: "default",
	profileName: "",
	profileTotal: 0,
	profiles: [],
	query: "",
	selectedUuid: "",
	textures: [],
	texturesLoading: false,
	textureType: "skin",
	visibility: "private",
};

function reducer(
	state: MinecraftProfilesPageState,
	action: MinecraftProfilesPageAction,
): MinecraftProfilesPageState {
	switch (action.type) {
		case "file":
		case "loading":
		case "model":
		case "profileName":
		case "query":
		case "selectedUuid":
		case "textures":
		case "texturesLoading":
		case "textureType":
		case "visibility":
			return { ...state, [action.type]: action.value };
		case "profiles":
			return {
				...state,
				profileTotal: action.value.length,
				profiles: action.value,
				selectedUuid: action.value.some(
					(profile) => profile.id === state.selectedUuid,
				)
					? state.selectedUuid
					: action.value[0]?.id || "",
			};
		case "profilePage":
			return {
				...state,
				profileTotal: action.value.total,
				profiles: action.value.items,
				selectedUuid: action.value.items.some(
					(profile) => profile.id === state.selectedUuid,
				)
					? state.selectedUuid
					: action.value.items[0]?.id || "",
			};
	}
}

export function useMinecraftProfilesPageState() {
	return useReducer(reducer, initialState);
}

import { useReducer } from "react";
import type {
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	MinecraftWardrobeTextureMetadata,
	YggdrasilProfile,
} from "@/types/api";

export type TextureWardrobePageState = {
	activeTexture: MinecraftWardrobeTextureMetadata | null;
	deleteDialogOpen: boolean;
	deleteTexture: MinecraftWardrobeTextureMetadata | null;
	dialogOpen: boolean;
	editDialogOpen: boolean;
	editTexture: MinecraftWardrobeTextureMetadata | null;
	file: File | null;
	loading: boolean;
	model: MinecraftTextureModel;
	profileQuery: string;
	profiles: YggdrasilProfile[];
	query: string;
	selectedProfileId: string;
	submitting: boolean;
	textureTotal: number;
	textures: MinecraftWardrobeTextureMetadata[];
	textureType: MinecraftTextureType;
	uploadName: string;
	visibility: MinecraftTextureVisibility;
};

export type TextureWardrobePageAction =
	| { type: "activeTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "deleteDialogOpen"; value: boolean }
	| { type: "deleteTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "dialogOpen"; value: boolean }
	| { type: "editDialogOpen"; value: boolean }
	| { type: "editTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "file"; value: File | null }
	| {
			type: "loadSuccess";
			profiles: YggdrasilProfile[];
			textureTotal: number;
			textures: MinecraftWardrobeTextureMetadata[];
	  }
	| { type: "loading"; value: boolean }
	| { type: "model"; value: MinecraftTextureModel }
	| { type: "profilesSuccess"; profiles: YggdrasilProfile[] }
	| { type: "profileQuery"; value: string }
	| { type: "query"; value: string }
	| { type: "selectedProfileId"; value: string | ((current: string) => string) }
	| { type: "submitting"; value: boolean }
	| { type: "textureType"; value: MinecraftTextureType }
	| {
			type: "texturesSuccess";
			textureTotal: number;
			textures: MinecraftWardrobeTextureMetadata[];
	  }
	| { type: "uploadName"; value: string }
	| { type: "visibility"; value: MinecraftTextureVisibility }
	| { type: "prependTexture"; value: MinecraftWardrobeTextureMetadata }
	| { type: "replaceTexture"; value: MinecraftWardrobeTextureMetadata }
	| { type: "removeTexture"; id: number };

const initialState: TextureWardrobePageState = {
	activeTexture: null,
	deleteDialogOpen: false,
	deleteTexture: null,
	dialogOpen: false,
	editDialogOpen: false,
	editTexture: null,
	file: null,
	loading: true,
	model: "default",
	profileQuery: "",
	profiles: [],
	query: "",
	selectedProfileId: "",
	submitting: false,
	textureTotal: 0,
	textures: [],
	textureType: "skin",
	uploadName: "",
	visibility: "private",
};

function reducer(
	state: TextureWardrobePageState,
	action: TextureWardrobePageAction,
): TextureWardrobePageState {
	switch (action.type) {
		case "activeTexture":
		case "deleteDialogOpen":
		case "deleteTexture":
		case "dialogOpen":
		case "editDialogOpen":
		case "editTexture":
		case "file":
		case "loading":
		case "model":
		case "profileQuery":
		case "query":
		case "submitting":
		case "textureType":
		case "uploadName":
		case "visibility":
			return { ...state, [action.type]: action.value };
		case "loadSuccess":
			return {
				...state,
				loading: false,
				profiles: action.profiles,
				selectedProfileId:
					state.selectedProfileId || action.profiles[0]?.id || "",
				textureTotal: action.textureTotal,
				textures: action.textures,
			};
		case "profilesSuccess":
			return {
				...state,
				profiles: action.profiles,
				selectedProfileId:
					state.selectedProfileId || action.profiles[0]?.id || "",
			};
		case "texturesSuccess":
			return {
				...state,
				loading: false,
				textureTotal: action.textureTotal,
				textures: action.textures,
			};
		case "prependTexture":
			return {
				...state,
				textureTotal: state.textureTotal + 1,
				textures: [action.value, ...state.textures],
			};
		case "replaceTexture":
			return {
				...state,
				activeTexture:
					state.activeTexture?.id === action.value.id
						? action.value
						: state.activeTexture,
				deleteTexture:
					state.deleteTexture?.id === action.value.id
						? action.value
						: state.deleteTexture,
				editTexture:
					state.editTexture?.id === action.value.id
						? action.value
						: state.editTexture,
				textures: state.textures.map((texture) =>
					texture.id === action.value.id ? action.value : texture,
				),
			};
		case "removeTexture":
			return {
				...state,
				textureTotal: Math.max(0, state.textureTotal - 1),
				textures: state.textures.filter((texture) => texture.id !== action.id),
			};
		case "selectedProfileId":
			return {
				...state,
				selectedProfileId:
					typeof action.value === "function"
						? action.value(state.selectedProfileId)
						: action.value,
			};
	}
}

export function useTextureWardrobePageState() {
	return useReducer(reducer, initialState);
}

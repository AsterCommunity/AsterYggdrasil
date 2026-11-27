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
	model: MinecraftTextureModel;
	profileQuery: string;
	profiles: YggdrasilProfile[];
	query: string;
	selectedProfileId: string;
	submitting: boolean;
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
	| { type: "model"; value: MinecraftTextureModel }
	| { type: "profilesSuccess"; profiles: YggdrasilProfile[] }
	| { type: "profileQuery"; value: string }
	| { type: "query"; value: string }
	| { type: "selectedProfileId"; value: string | ((current: string) => string) }
	| { type: "submitting"; value: boolean }
	| { type: "textureType"; value: MinecraftTextureType }
	| { type: "uploadName"; value: string }
	| { type: "visibility"; value: MinecraftTextureVisibility }
	| { type: "replaceTexture"; value: MinecraftWardrobeTextureMetadata };

const initialState: TextureWardrobePageState = {
	activeTexture: null,
	deleteDialogOpen: false,
	deleteTexture: null,
	dialogOpen: false,
	editDialogOpen: false,
	editTexture: null,
	file: null,
	model: "default",
	profileQuery: "",
	profiles: [],
	query: "",
	selectedProfileId: "",
	submitting: false,
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
		case "model":
		case "profileQuery":
		case "query":
		case "submitting":
		case "textureType":
		case "uploadName":
		case "visibility":
			return { ...state, [action.type]: action.value };
		case "profilesSuccess":
			return {
				...state,
				profiles: action.profiles,
				selectedProfileId:
					state.selectedProfileId || action.profiles[0]?.id || "",
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

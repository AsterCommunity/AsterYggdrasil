import { useCallback, useEffect, useReducer } from "react";
import { authService } from "@/services/authService";

type CaptchaState = {
	answer: string;
	challengeId: string | null;
	error: string | null;
	imageBase64: string | null;
	loading: boolean;
};

type CaptchaAction =
	| { type: "answer"; value: string }
	| { type: "loading" }
	| { type: "loaded"; challengeId: string; imageBase64: string }
	| { type: "error"; message: string }
	| { type: "clear" };

const initialState: CaptchaState = {
	answer: "",
	challengeId: null,
	error: null,
	imageBase64: null,
	loading: false,
};

function reducer(state: CaptchaState, action: CaptchaAction): CaptchaState {
	switch (action.type) {
		case "answer":
			return { ...state, answer: action.value };
		case "loading":
			return { ...state, answer: "", error: null, loading: true };
		case "loaded":
			return {
				answer: "",
				challengeId: action.challengeId,
				error: null,
				imageBase64: action.imageBase64,
				loading: false,
			};
		case "error":
			return {
				...state,
				answer: "",
				challengeId: null,
				error: action.message,
				imageBase64: null,
				loading: false,
			};
		case "clear":
			return initialState;
	}
}

export function useCaptchaChallenge(enabled: boolean) {
	const [state, dispatch] = useReducer(reducer, initialState);

	const refresh = useCallback(async () => {
		if (!enabled) {
			dispatch({ type: "clear" });
			return;
		}
		dispatch({ type: "loading" });
		try {
			const challenge = await authService.issueCaptcha();
			dispatch({
				type: "loaded",
				challengeId: challenge.challenge_id,
				imageBase64: challenge.image_base64,
			});
		} catch (error) {
			dispatch({
				type: "error",
				message: error instanceof Error ? error.message : "captcha load failed",
			});
		}
	}, [enabled]);

	useEffect(() => {
		if (!enabled) {
			dispatch({ type: "clear" });
			return;
		}
		void refresh();
	}, [enabled, refresh]);

	return {
		...state,
		refresh,
		setAnswer: (value: string) => dispatch({ type: "answer", value }),
	};
}

import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { resources } from "@/i18n/resources";
import { readStorageItem, STORAGE_KEYS, writeStorageItem } from "@/lib/storage";

const supportedLanguages = Object.keys(resources);

/** The single namespace all static and backend-installed bundles merge into. */
export const DEFAULT_I18N_NAMESPACE = "frontend";

function supportedLanguageFor(value: string | null | undefined) {
	if (!value) {
		return null;
	}
	if (supportedLanguages.includes(value)) {
		return value;
	}
	if (value.toLowerCase().startsWith("zh")) {
		return "zh-CN";
	}
	return null;
}

export function detectLanguage() {
	const stored = supportedLanguageFor(
		readStorageItem("local", STORAGE_KEYS.languagePreference),
	);
	if (stored) {
		return stored;
	}

	if (typeof navigator === "undefined") {
		return "en-US";
	}
	const preferred = navigator.languages?.[0] ?? navigator.language;
	return supportedLanguageFor(preferred) ?? "en-US";
}

export function persistLanguagePreference(language: string) {
	const nextLanguage = supportedLanguageFor(language);
	if (!nextLanguage) {
		return false;
	}
	return writeStorageItem(
		"local",
		STORAGE_KEYS.languagePreference,
		nextLanguage,
	);
}

void i18next.use(initReactI18next).init({
	resources,
	lng: detectLanguage(),
	fallbackLng: "en-US",
	defaultNS: DEFAULT_I18N_NAMESPACE,
	interpolation: {
		escapeValue: false,
	},
});

export { i18next };

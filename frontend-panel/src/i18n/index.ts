import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { resources } from "@/i18n/resources";
import { readStorageItem, STORAGE_KEYS, writeStorageItem } from "@/lib/storage";

const supportedLanguages = Object.keys(resources);

/**
 * BCP 47 document tag for each supported UI language; index.html hardcodes
 * `lang="en"`, so the active locale has to be mirrored onto <html lang> here.
 */
const DOCUMENT_LANGUAGE_TAGS = {
	"en-US": "en-US",
	"zh-CN": "zh-CN",
} as const satisfies Record<string, string>;

export function documentLanguageTag(language: string) {
	const tag =
		DOCUMENT_LANGUAGE_TAGS[language as keyof typeof DOCUMENT_LANGUAGE_TAGS];
	if (tag) {
		return tag;
	}
	return language.split("-")[0] || language;
}

function syncDocumentLanguage(language: string) {
	if (typeof document === "undefined") {
		return;
	}
	document.documentElement.lang = documentLanguageTag(language);
}

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
	defaultNS: "frontend",
	interpolation: {
		escapeValue: false,
	},
});

i18next.on("languageChanged", syncDocumentLanguage);
syncDocumentLanguage(i18next.language);

export { i18next };

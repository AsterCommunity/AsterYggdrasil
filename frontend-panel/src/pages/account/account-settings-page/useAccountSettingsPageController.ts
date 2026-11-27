import { useEffect, useMemo, useRef, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
	clearContactVerificationRedirectSearch,
	getContactVerificationRedirectState,
} from "@/lib/contactVerificationRedirect";
import {
	SETTINGS_SECTION_IDS,
	type SectionDefinition,
	type SettingsSectionId,
	settingSectionElementId,
} from "@/pages/account/account-settings-page/settingsSection";
import { accountPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";

type ActiveSettingsSectionStore = {
	getSnapshot: () => SettingsSectionId;
	set: (next: SettingsSectionId) => void;
	subscribe: (listener: () => void) => () => void;
};

function initialActiveSettingsSection(): SettingsSectionId {
	if (typeof window === "undefined") {
		return "profile";
	}
	if (window.location.pathname === accountPaths.settingsSecurityCompat) {
		return "security";
	}
	const hash = window.location.hash.replace("#", "");
	return SETTINGS_SECTION_IDS.includes(hash as SettingsSectionId)
		? (hash as SettingsSectionId)
		: "profile";
}

function createActiveSettingsSectionStore(
	initialValue: SettingsSectionId,
): ActiveSettingsSectionStore {
	let snapshot = initialValue;
	const listeners = new Set<() => void>();
	return {
		getSnapshot: () => snapshot,
		set: (next) => {
			if (next === snapshot) return;
			snapshot = next;
			for (const listener of listeners) listener();
		},
		subscribe: (listener) => {
			listeners.add(listener);
			return () => listeners.delete(listener);
		},
	};
}

function useActiveSettingsSection() {
	const storeRef = useRef<ActiveSettingsSectionStore | null>(null);
	if (storeRef.current === null) {
		storeRef.current = createActiveSettingsSectionStore(
			initialActiveSettingsSection(),
		);
	}
	const store = storeRef.current;
	const activeSection = useSyncExternalStore(
		store.subscribe,
		store.getSnapshot,
		(): SettingsSectionId => "profile",
	);

	useEffect(() => {
		const hash = window.location.hash.replace("#", "");
		if (SETTINGS_SECTION_IDS.includes(hash as SettingsSectionId)) {
			const id = hash as SettingsSectionId;
			window.requestAnimationFrame(() => {
				document
					.getElementById(settingSectionElementId(id))
					?.scrollIntoView({ block: "start" });
			});
		}
	}, []);

	useEffect(() => {
		const elements = SETTINGS_SECTION_IDS.map((id) =>
			document.getElementById(settingSectionElementId(id)),
		).filter((element): element is HTMLElement => Boolean(element));
		if (elements.length === 0) return;

		const observer = new IntersectionObserver(
			(entries) => {
				const visible = entries.reduce<IntersectionObserverEntry | null>(
					(best, entry) => {
						if (!entry.isIntersecting) return best;
						if (!best || entry.intersectionRatio > best.intersectionRatio) {
							return entry;
						}
						return best;
					},
					null,
				);
				const id = visible?.target.getAttribute(
					"data-settings-section",
				) as SettingsSectionId | null;
				if (id) store.set(id);
			},
			{
				root: null,
				rootMargin: "-18% 0px -62% 0px",
				threshold: [0.2, 0.4, 0.7],
			},
		);

		for (const element of elements) observer.observe(element);
		return () => observer.disconnect();
	}, [store]);

	return activeSection;
}

export function useAccountSettingsPageController() {
	const { t } = useTranslation();
	const user = useAuthStore((state) => state.user);
	const refreshUser = useAuthStore((state) => state.refreshUser);
	const location = useLocation();
	const navigate = useNavigate();
	const locationSearch = location.search;
	const activeSection = useActiveSettingsSection();

	useEffect(() => {
		const redirect = getContactVerificationRedirectState(locationSearch);
		if (!redirect) return;
		if (redirect.status === "email-changed") {
			toast.success(
				redirect.email
					? t("personalSettings.emailChangeConfirmedWithEmail", {
							email: redirect.email,
						})
					: t("personalSettings.emailChangeConfirmed"),
			);
			void refreshUser();
		}

		navigate(
			{
				pathname: accountPaths.settings,
				search: clearContactVerificationRedirectSearch(locationSearch),
				hash: "security",
			},
			{ replace: true },
		);
	}, [locationSearch, navigate, refreshUser, t]);

	const sections = useMemo<SectionDefinition[]>(
		() => [
			{
				id: "profile",
				icon: "User",
				label: t("personalSettings.sections.profile"),
				description: t("personalSettings.sectionDescriptions.profile"),
			},
			{
				id: "security",
				icon: "Shield",
				label: t("personalSettings.sections.security"),
				description: t("personalSettings.sectionDescriptions.security"),
			},
			{
				id: "passkeys",
				icon: "Key",
				label: t("personalSettings.sections.passkeys"),
				description: t("personalSettings.sectionDescriptions.passkeys"),
			},
			{
				id: "external-auth",
				icon: "LinkSimple",
				label: t("personalSettings.sections.externalAuth"),
				description: t("personalSettings.sectionDescriptions.externalAuth"),
			},
		],
		[t],
	);

	return { activeSection, sections, user };
}

import { useRegisterSW } from "virtual:pwa-register/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { logger } from "@/lib/logger";

function logPwaUpdate(message: string, extra?: unknown) {
	if (extra === undefined) {
		logger.debug(`[pwa-update] ${message}`);
		return;
	}
	logger.debug(`[pwa-update] ${message}`, extra);
}

function requestServiceWorkerUpdate(
	registration: ServiceWorkerRegistration,
	reason: string,
) {
	logPwaUpdate("update check", reason);
	void registration.update().catch((error: unknown) => {
		logPwaUpdate("update check failed", error);
	});
}

export function usePwaUpdate() {
	const { t } = useTranslation();
	const [registration, setRegistration] =
		useState<ServiceWorkerRegistration | null>(null);
	const {
		needRefresh: [needRefresh],
		offlineReady: [offlineReady],
		updateServiceWorker,
	} = useRegisterSW({
		onRegistered(registration) {
			logPwaUpdate("registered", {
				active: registration?.active?.scriptURL,
				installing: registration?.installing?.scriptURL,
				scope: registration?.scope,
				waiting: registration?.waiting?.scriptURL,
			});

			if (!registration) return;
			setRegistration(registration);
		},
		onRegisterError(error) {
			logPwaUpdate("register error", error);
		},
	});

	useEffect(() => {
		if (!registration) return;

		requestServiceWorkerUpdate(registration, "registered");

		const interval = window.setInterval(
			() => {
				requestServiceWorkerUpdate(registration, "interval");
			},
			60 * 60 * 1000,
		);

		const handleVisibilityChange = () => {
			if (document.visibilityState !== "visible") return;
			requestServiceWorkerUpdate(registration, "visibility");
		};

		document.addEventListener("visibilitychange", handleVisibilityChange);

		return () => {
			window.clearInterval(interval);
			document.removeEventListener("visibilitychange", handleVisibilityChange);
		};
	}, [registration]);

	useEffect(() => {
		logPwaUpdate("needRefresh changed", needRefresh);
	}, [needRefresh]);

	useEffect(() => {
		logPwaUpdate("offlineReady changed", offlineReady);
	}, [offlineReady]);

	useEffect(() => {
		if (!needRefresh) return;

		toast.info(t("pwa.updateAvailable"), {
			action: {
				label: t("pwa.refresh"),
				onClick: () => {
					logPwaUpdate("apply update clicked");
					void updateServiceWorker(true);
				},
			},
			duration: Number.POSITIVE_INFINITY,
		});
	}, [needRefresh, t, updateServiceWorker]);
}

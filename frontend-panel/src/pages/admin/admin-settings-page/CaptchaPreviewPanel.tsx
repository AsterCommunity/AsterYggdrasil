import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { RefreshButton } from "@/components/common/RefreshButton";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { adminConfigService } from "@/services/adminService";
import type { SystemConfig } from "@/types/api";
import { formatError } from "./settingsCategories";
import { buildCaptchaPreviewValues, type DraftValue } from "./settingsDraft";

export function CaptchaPreviewPanel({
	configs,
	drafts,
}: {
	configs: SystemConfig[];
	drafts: Record<string, DraftValue>;
}) {
	const { t } = useTranslation();
	const [imageBase64, setImageBase64] = useState<string | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const previewValues = useMemo(
		() => buildCaptchaPreviewValues(configs, drafts),
		[configs, drafts],
	);
	const previewValuesRef = useRef(previewValues);

	useEffect(() => {
		previewValuesRef.current = previewValues;
	}, [previewValues]);

	const refresh = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const result = await adminConfigService.previewCaptcha(
				previewValuesRef.current,
			);
			setImageBase64(result.value ?? null);
		} catch (nextError) {
			setImageBase64(null);
			setError(formatError(nextError));
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		void refresh();
	}, [refresh]);

	return (
		<div className="grid gap-3 px-4 py-4 lg:grid-cols-[minmax(16rem,0.72fr)_minmax(0,1fr)] lg:items-start">
			<div className="min-w-0">
				<div className="flex flex-wrap items-center gap-2">
					<Label className="text-sm font-semibold">
						{t("settings_auth_captcha_preview_label")}
					</Label>
				</div>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("settings_auth_captcha_preview_desc")}
				</p>
			</div>
			<div className="flex min-w-0 flex-col gap-3 sm:flex-row sm:items-center">
				<div className="flex h-16 w-full max-w-[13.5rem] items-center justify-start overflow-hidden bg-transparent">
					{loading ? (
						<Skeleton className="h-14 w-full max-w-[13.5rem] rounded-lg" />
					) : imageBase64 ? (
						<img
							src={imageBase64}
							alt={t("login.captchaImageAlt")}
							className="h-full max-w-full object-contain"
							draggable={false}
						/>
					) : (
						<span className="text-xs text-muted-foreground">
							{error ?? t("login.captchaUnavailable")}
						</span>
					)}
				</div>
				<RefreshButton loading={loading} onClick={() => void refresh()}>
					{t("settings_auth_captcha_preview_refresh")}
				</RefreshButton>
			</div>
		</div>
	);
}

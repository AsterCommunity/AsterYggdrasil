import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type SaveBarPhase = "hidden" | "entering" | "visible" | "exiting";
type SaveBarPhaseState = {
	active: boolean;
	phase: SaveBarPhase;
};

const SAVE_BAR_ENTER_DURATION_MS = 150;
const SAVE_BAR_EXIT_DURATION_MS = 140;
const SAVE_BAR_EXIT_UNMOUNT_GRACE_MS = 50;

export function SettingsSaveBar({
	changedCount,
	disabled,
	error,
	hasUnsavedChanges,
	onDiscard,
	onSave,
	saving,
}: {
	changedCount: number;
	disabled: boolean;
	error: string | null;
	hasUnsavedChanges: boolean;
	onDiscard: () => void;
	onSave: () => void;
	saving: boolean;
}) {
	const { t } = useTranslation();
	const active = hasUnsavedChanges || Boolean(error);
	const { phase, transitionDurationMs } = useSettingsSaveBarPhase(active);
	const latestVisibleStateRef = useRef({
		changedCount,
		disabled,
		error,
		hasUnsavedChanges,
		saving,
	});

	if (phase === "hidden") return null;

	if (active) {
		latestVisibleStateRef.current = {
			changedCount,
			disabled,
			error,
			hasUnsavedChanges,
			saving,
		};
	}

	const displayState =
		phase === "exiting"
			? latestVisibleStateRef.current
			: {
					changedCount,
					disabled,
					error,
					hasUnsavedChanges,
					saving,
				};
	const actionsDisabled =
		phase === "exiting" ||
		displayState.saving ||
		!displayState.hasUnsavedChanges;

	return (
		<div
			aria-hidden={!active || phase === "exiting"}
			data-testid="settings-save-bar"
			data-phase={phase}
			className="pointer-events-none sticky bottom-4 z-20"
		>
			<div
				className={cn(
					"mx-auto w-full max-w-4xl origin-bottom transition-[opacity,transform] will-change-transform motion-reduce:transition-none",
					phase === "entering"
						? "pointer-events-none translate-y-2 opacity-0 ease-out"
						: phase === "visible"
							? "pointer-events-auto translate-y-0 opacity-100 ease-out"
							: "pointer-events-none translate-y-0 opacity-0 ease-out",
				)}
				style={{ transitionDuration: `${transitionDurationMs}ms` }}
			>
				<AdminSurface
					className={cn(
						"flex flex-col gap-3 bg-background/95 shadow-2xl shadow-black/10 ring-1 backdrop-blur-xl dark:bg-card/95 dark:shadow-none sm:flex-row sm:items-center sm:justify-between",
						displayState.error
							? "border-destructive/40 ring-destructive/10"
							: "border-emerald-500/35 ring-border/50",
					)}
				>
					<div className="min-w-0">
						<div className="text-sm font-semibold">
							{displayState.error
								? t("settings_save_failed")
								: t("settings_save_notice", {
										count: displayState.changedCount,
									})}
						</div>
						<p
							className={cn(
								"mt-1 text-xs text-muted-foreground",
								displayState.error && "text-destructive",
							)}
						>
							{displayState.error ?? t("settings_save_hint")}
						</p>
					</div>
					<div className="flex shrink-0 flex-wrap gap-2">
						<Button
							type="button"
							variant="outline"
							disabled={actionsDisabled}
							onClick={onDiscard}
						>
							{t("undo_changes")}
						</Button>
						<Button
							type="button"
							disabled={actionsDisabled || displayState.disabled}
							onClick={onSave}
						>
							{displayState.saving ? t("settings_saving") : t("save_changes")}
						</Button>
					</div>
				</AdminSurface>
			</div>
		</div>
	);
}

function useSettingsSaveBarPhase(active: boolean) {
	const [state, setState] = useState<SaveBarPhaseState>(() => ({
		active,
		phase: active ? "entering" : "hidden",
	}));
	let phase = state.phase;

	if (active !== state.active) {
		phase = active
			? "entering"
			: state.phase === "hidden"
				? "hidden"
				: "exiting";
		setState({ active, phase });
	}

	useEffect(() => {
		if (phase === "entering") {
			const timer = window.setTimeout(() => {
				setState((current) =>
					current.phase === "entering"
						? { ...current, phase: "visible" }
						: current,
				);
			}, 0);
			return () => window.clearTimeout(timer);
		}

		if (phase === "exiting") {
			const timer = window.setTimeout(() => {
				setState((current) =>
					current.phase === "exiting"
						? { ...current, phase: "hidden" }
						: current,
				);
			}, SAVE_BAR_EXIT_DURATION_MS + SAVE_BAR_EXIT_UNMOUNT_GRACE_MS);
			return () => window.clearTimeout(timer);
		}
	}, [phase]);

	return {
		phase,
		transitionDurationMs:
			phase === "exiting"
				? SAVE_BAR_EXIT_DURATION_MS
				: SAVE_BAR_ENTER_DURATION_MS,
	};
}

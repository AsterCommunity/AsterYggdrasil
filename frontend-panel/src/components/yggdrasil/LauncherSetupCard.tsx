import type { DragEvent } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon, type IconName } from "@/components/ui/icon";
import { CopyField } from "@/components/yggdrasil/CopyField";
import { cn } from "@/lib/utils";
import {
	yggdrasilAddServerUri,
	yggdrasilApiRoot,
} from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type LauncherSetupCardProps = {
	className?: string;
	profileName?: string | null;
	showServerOwner?: boolean;
};

export function LauncherSetupCard({
	className,
	profileName,
	showServerOwner = false,
}: LauncherSetupCardProps) {
	const { t } = useTranslation();
	const yggdrasilConfig = useFrontendConfigStore((state) => state.yggdrasil);
	const [copiedDragUri, setCopiedDragUri] = useState(false);
	const [serverOwnerExpanded, setServerOwnerExpanded] = useState(false);
	const apiRoot = yggdrasilApiRoot(yggdrasilConfig);
	const addServerUri = yggdrasilAddServerUri(apiRoot);
	const serverJvmArg = `-javaagent:authlib-injector.jar=${apiRoot}`;
	const serverCommand = `java ${serverJvmArg} -jar minecraft_server.jar nogui`;
	const secureProfileProperties =
		"online-mode=true\nenforce-secure-profile=true";

	function handleDragStart(event: DragEvent<HTMLElement>) {
		event.dataTransfer.setData("text/plain", addServerUri);
		event.dataTransfer.dropEffect = "copy";
		event.dataTransfer.effectAllowed = "copy";
	}

	async function copyAddServerUri() {
		await navigator.clipboard.writeText(addServerUri);
		setCopiedDragUri(true);
		window.setTimeout(() => setCopiedDragUri(false), 1200);
	}

	return (
		<section
			className={cn(
				"launcher-setup-card overflow-hidden rounded-xl border border-emerald-900/10 bg-white/82 text-card-foreground shadow-xl shadow-emerald-950/5 backdrop-blur dark:border-white/10 dark:bg-white/[0.055] dark:shadow-black/20",
				className,
			)}
		>
			<div className="border-b border-emerald-900/10 p-5 dark:border-white/10">
				<div className="min-w-0">
					<div className="min-w-0">
						<div className="flex items-center gap-2 text-xs font-semibold text-emerald-700 dark:text-emerald-300">
							<Icon name="Monitor" className="size-5 shrink-0" />
							{t("launcher.protocolBadge")}
						</div>
						<h2 className="mt-3 text-xl font-bold tracking-normal">
							{profileName
								? t("launcher.profileTitle")
								: t("launcher.userTitle")}
						</h2>
						<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
							{profileName
								? t("launcher.profileHint")
								: t("launcher.userDescription")}
						</p>
					</div>
				</div>
			</div>

			<div className="launcher-setup-card__body grid gap-0">
				<div className="min-w-0 p-5">
					<div className="grid gap-4">
						<div className="grid gap-3 border-b border-emerald-900/10 pb-4 dark:border-white/10">
							<LauncherStep
								icon="SignIn"
								title={t("launcher.externalLoginName")}
								description={t("launcher.externalLoginDescription")}
							/>
							<LauncherStep
								icon="User"
								title={
									profileName
										? t("launcher.selectProfileName")
										: t("launcher.selectProfile")
								}
								description={t("launcher.selectProfileDescription")}
							/>
						</div>

						<CopyField label="API Root" value={apiRoot} compact />
					</div>
				</div>

				<div className="launcher-setup-card__drag-panel border-t border-emerald-900/10 p-5 dark:border-white/10">
					<button
						type="button"
						draggable
						onDragStart={handleDragStart}
						onClick={() => void copyAddServerUri()}
						className="launcher-setup-card__drag-button group flex w-full cursor-grab select-none items-center justify-center gap-2 rounded-lg border border-dashed border-emerald-400/70 bg-emerald-50/72 p-3 text-center text-sm font-semibold text-emerald-950 transition-[border-color,background-color,transform] hover:-translate-y-0.5 hover:border-emerald-500 hover:bg-emerald-50 active:cursor-grabbing dark:border-emerald-300/30 dark:bg-emerald-400/10 dark:text-emerald-50 dark:hover:bg-emerald-400/14"
					>
						<Icon
							name={copiedDragUri ? "Check" : "Monitor"}
							className="launcher-setup-card__drag-icon size-5 shrink-0 text-emerald-700 transition-transform group-hover:scale-105 dark:text-emerald-200"
						/>
						<div className="font-semibold">
							{copiedDragUri
								? t("common.copied")
								: t("launcher.dragToLauncher")}
						</div>
						<p className="launcher-setup-card__drag-hint mx-auto max-w-52 text-xs leading-5 text-emerald-900/70 dark:text-emerald-100/68">
							{t("launcher.dragHint")}
						</p>
					</button>
					<div className="mt-3 flex items-start gap-2 text-xs leading-5 text-muted-foreground">
						<Icon name="Info" className="mt-0.5 size-4 shrink-0" />
						<span>{t("launcher.apiRootHint")}</span>
					</div>
				</div>
			</div>

			{showServerOwner ? (
				<div className="border-t border-emerald-900/10 bg-muted/18 text-sm dark:border-white/10 dark:bg-black/10">
					<button
						type="button"
						aria-expanded={serverOwnerExpanded}
						aria-controls="launcher-server-owner-panel"
						onClick={() => setServerOwnerExpanded((expanded) => !expanded)}
						className="flex w-full items-center justify-between gap-3 p-5 text-left font-medium transition-colors hover:bg-emerald-50/45 dark:hover:bg-emerald-400/6"
					>
						<span>{t("launcher.serverOwnerTitle")}</span>
						<Icon
							name="CaretDown"
							className={cn(
								"size-4 shrink-0 text-muted-foreground transition-transform duration-200 ease-out motion-reduce:transition-none",
								serverOwnerExpanded && "rotate-180",
							)}
						/>
					</button>
					<div
						id="launcher-server-owner-panel"
						className={cn(
							"grid transition-[grid-template-rows,opacity] duration-240 ease-out motion-reduce:transition-none",
							serverOwnerExpanded
								? "grid-rows-[1fr] opacity-100"
								: "grid-rows-[0fr] opacity-0",
						)}
					>
						<div className="min-h-0 overflow-hidden">
							<div className="grid gap-3 px-5 pb-5 text-xs leading-5 text-muted-foreground">
								<p>{t("launcher.serverOwnerDescription")}</p>
								<CopyField
									label={t("launcher.jvmArg")}
									value={serverJvmArg}
									compact
								/>
								<CopyField
									label={t("launcher.serverCommand")}
									value={serverCommand}
									compact
								/>
								<div className="grid grid-cols-[1rem_minmax(0,1fr)] gap-x-3 gap-y-3 rounded-lg border border-emerald-900/10 bg-white/68 p-3 text-foreground shadow-xs dark:border-white/10 dark:bg-white/[0.045]">
									<div className="pt-0.5">
										<Icon
											name="Shield"
											className="size-4 text-emerald-700 dark:text-emerald-300"
										/>
									</div>
									<div className="min-w-0">
										<div className="font-semibold">
											{t("launcher.secureProfileTitle")}
										</div>
										<p className="mt-1 text-muted-foreground">
											{t("launcher.secureProfileDescription")}
										</p>
									</div>
									<div className="col-start-2 grid min-w-0 gap-3">
										<pre className="overflow-x-auto rounded-md border border-input bg-background/70 px-3 py-2 font-mono text-[11px] leading-5 text-foreground">
											{secureProfileProperties}
										</pre>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			) : null}
		</section>
	);
}

function LauncherStep({
	description,
	icon,
	title,
}: {
	description: string;
	icon: IconName;
	title: string;
}) {
	return (
		<div className="flex items-start gap-3">
			<Icon
				name={icon}
				className="mt-0.5 size-5 shrink-0 text-emerald-700 dark:text-emerald-200"
			/>
			<div className="min-w-0">
				<div className="text-sm font-semibold">{title}</div>
				<div className="mt-0.5 text-xs leading-5 text-muted-foreground">
					{description}
				</div>
			</div>
		</div>
	);
}

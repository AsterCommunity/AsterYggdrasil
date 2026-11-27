import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { ConfigSchemaItem, SystemConfig } from "@/types/api";
import { SettingControl } from "./SettingControls";
import { translateOrFallback } from "./settingsCategories";
import {
	type DraftValue,
	draftEqualsConfig,
	humanizeKey,
} from "./settingsDraft";

export function SettingRow({
	config,
	draft,
	onChange,
	schema,
	templateVariableAction,
}: {
	config: SystemConfig;
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	schema?: ConfigSchemaItem;
	templateVariableAction?: {
		disabled: boolean;
		onClick: () => void;
	};
}) {
	const { t } = useTranslation();
	const label = translateOrFallback(
		t,
		schema?.label_i18n_key,
		humanizeKey(config.key),
	);
	const description = translateOrFallback(
		t,
		schema?.description_i18n_key,
		config.description,
	);
	const changed = !draftEqualsConfig(config, draft);

	return (
		<div className="grid gap-3 px-4 py-4 lg:grid-cols-[minmax(16rem,0.72fr)_minmax(0,1fr)] lg:items-start">
			<div className="min-w-0">
				<div className="flex flex-wrap items-center gap-2">
					<Label className="text-sm font-semibold">{label}</Label>
					<SettingDescriptionHelp
						description={description}
						label={t("settings_config_description_help", { label })}
					/>
					{changed ? (
						<span className="text-xs font-medium text-primary">
							{t("settings_status_unsaved")}
						</span>
					) : null}
					{config.is_sensitive ? (
						<span className="text-xs text-muted-foreground">
							{t("settings_status_sensitive")}
						</span>
					) : null}
					{config.requires_restart ? (
						<span className="text-xs text-muted-foreground">
							{t("requires_restart")}
						</span>
					) : null}
				</div>
				{templateVariableAction ? (
					<button
						type="button"
						disabled={templateVariableAction.disabled}
						className="mt-2 w-fit text-sm text-primary underline-offset-4 transition-colors hover:text-primary/80 hover:underline disabled:pointer-events-none disabled:text-muted-foreground"
						onClick={templateVariableAction.onClick}
					>
						{t("mail_template_variable_link")}
					</button>
				) : null}
			</div>
			<div className="min-w-0">
				<SettingControl
					config={config}
					draft={draft}
					schema={schema}
					onChange={onChange}
				/>
			</div>
		</div>
	);
}

function SettingDescriptionHelp({
	description,
	label,
}: {
	description: string;
	label: string;
}) {
	if (!description.trim()) return null;

	return (
		<TooltipProvider delay={0}>
			<Tooltip>
				<TooltipTrigger
					type="button"
					aria-label={label}
					className="inline-flex size-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-muted-foreground transition-colors hover:bg-accent/55 hover:text-foreground focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35"
				>
					<span aria-hidden="true">?</span>
				</TooltipTrigger>
				<TooltipContent
					side="top"
					align="start"
					className="max-w-[min(24rem,calc(100vw-2rem))] whitespace-normal break-words leading-5"
				>
					{description}
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}

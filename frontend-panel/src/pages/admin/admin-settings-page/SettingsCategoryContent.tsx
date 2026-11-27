import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { AnimatedCollapsible } from "@/components/common/AnimatedCollapsible";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ConfigSchemaItem, SystemConfig } from "@/types/api";
import { CaptchaPreviewPanel } from "./CaptchaPreviewPanel";
import { SettingRow } from "./SettingRow";
import {
	formatSubcategoryDescription,
	formatSubcategoryLabel,
	rootCategory,
} from "./settingsCategories";
import {
	configToDraft,
	type DraftValue,
	draftEqualsConfig,
} from "./settingsDraft";
import {
	buildMailTemplateGroups,
	formatMailTemplateGroupLabel,
	type MailTemplateGroupItem,
} from "./settingsMailTemplates";

export function SettingsCategoryContent({
	drafts,
	empty,
	expandedTemplateGroups,
	groupedConfigs,
	loading,
	onChange,
	onOpenTemplateVariables,
	onOpenTestEmail,
	onRotateYggdrasilSignatureKey,
	onToggleTemplateGroup,
	rotatingYggdrasilKey,
	schemaMap,
}: {
	drafts: Record<string, DraftValue>;
	empty: boolean;
	expandedTemplateGroups: Record<string, boolean>;
	groupedConfigs: Record<string, SystemConfig[]>;
	loading: boolean;
	onChange: (key: string, draft: DraftValue) => void;
	onOpenTemplateVariables: (templateCode: string) => void;
	onOpenTestEmail: () => void;
	onRotateYggdrasilSignatureKey: () => void;
	onToggleTemplateGroup: (groupKey: string, open: boolean) => void;
	rotatingYggdrasilKey: boolean;
	schemaMap: Map<string, ConfigSchemaItem>;
}) {
	const { t } = useTranslation();

	if (loading) {
		return (
			<section className="min-w-0">
				<SettingsSkeleton />
			</section>
		);
	}

	if (empty) {
		return (
			<section className="min-w-0">
				<AdminSurface padded={false}>
					<div className="grid min-h-56 place-items-center px-4 py-10 text-center">
						<div className="max-w-md">
							<div className="text-sm font-semibold">
								{t("settings_empty_title")}
							</div>
							<p className="mt-1 text-sm leading-6 text-muted-foreground">
								{t("settings_empty_desc")}
							</p>
						</div>
					</div>
				</AdminSurface>
			</section>
		);
	}

	return (
		<section className="min-w-0">
			<div className="grid gap-4">
				{Object.entries(groupedConfigs).map(([category, items]) => (
					<SettingsGroup
						key={category}
						category={category}
						configs={items}
						drafts={drafts}
						schemaMap={schemaMap}
						expandedTemplateGroups={expandedTemplateGroups}
						rotatingYggdrasilKey={rotatingYggdrasilKey}
						onChange={onChange}
						onOpenTemplateVariables={onOpenTemplateVariables}
						onOpenTestEmail={onOpenTestEmail}
						onRotateYggdrasilSignatureKey={onRotateYggdrasilSignatureKey}
						onToggleTemplateGroup={onToggleTemplateGroup}
					/>
				))}
			</div>
		</section>
	);
}

function SettingsGroup({
	category,
	configs,
	drafts,
	expandedTemplateGroups,
	onChange,
	onOpenTemplateVariables,
	onOpenTestEmail,
	onRotateYggdrasilSignatureKey,
	onToggleTemplateGroup,
	rotatingYggdrasilKey,
	schemaMap,
}: {
	category: string;
	configs: SystemConfig[];
	drafts: Record<string, DraftValue>;
	expandedTemplateGroups: Record<string, boolean>;
	onChange: (key: string, draft: DraftValue) => void;
	onOpenTemplateVariables: (templateCode: string) => void;
	onOpenTestEmail: () => void;
	onRotateYggdrasilSignatureKey: () => void;
	onToggleTemplateGroup: (groupKey: string, open: boolean) => void;
	rotatingYggdrasilKey: boolean;
	schemaMap: Map<string, ConfigSchemaItem>;
}) {
	const { t } = useTranslation();
	const root = rootCategory(category);
	const isMailTemplateSection = category === "mail.template";
	const action = getSettingsGroupAction({
		category,
		onOpenTestEmail,
		onRotateYggdrasilSignatureKey,
		rotatingYggdrasilKey,
		t,
	});
	const templateGroups = isMailTemplateSection
		? buildMailTemplateGroups(category, configs)
		: [];

	return (
		<AdminSurface padded={false} className="overflow-hidden">
			<div className="flex flex-col gap-3 border-b border-border/70 px-4 py-3 dark:border-white/10 lg:flex-row lg:items-start lg:justify-between">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold">
						{formatSubcategoryLabel(root, category, t)}
					</h3>
					<p className="mt-1 text-sm leading-6 text-muted-foreground">
						{formatSubcategoryDescription(root, category, t)}
					</p>
				</div>
				{action}
			</div>
			{isMailTemplateSection ? (
				<div className="grid gap-3 p-4">
					{templateGroups.map((group) => (
						<MailTemplateGroup
							key={group.groupKey}
							changedCount={
								group.configs.filter((config) => {
									const draft = drafts[config.key];
									return draft ? !draftEqualsConfig(config, draft) : false;
								}).length
							}
							drafts={drafts}
							group={group}
							open={expandedTemplateGroups[group.groupKey] ?? false}
							schemaMap={schemaMap}
							onChange={onChange}
							onOpenTemplateVariables={onOpenTemplateVariables}
							onToggle={(open) => onToggleTemplateGroup(group.groupKey, open)}
						/>
					))}
				</div>
			) : (
				<div className="divide-y divide-border/70 dark:divide-white/10">
					{category === "auth.captcha" ? (
						<CaptchaPreviewPanel configs={configs} drafts={drafts} />
					) : null}
					{configs.map((config) => (
						<SettingRow
							key={config.key}
							config={config}
							draft={drafts[config.key] ?? configToDraft(config)}
							schema={schemaMap.get(config.key)}
							onChange={(draft) => onChange(config.key, draft)}
						/>
					))}
				</div>
			)}
		</AdminSurface>
	);
}

function MailTemplateGroup({
	changedCount,
	drafts,
	group,
	onChange,
	onOpenTemplateVariables,
	onToggle,
	open,
	schemaMap,
}: {
	changedCount: number;
	drafts: Record<string, DraftValue>;
	group: MailTemplateGroupItem;
	onChange: (key: string, draft: DraftValue) => void;
	onOpenTemplateVariables: (templateCode: string) => void;
	onToggle: (open: boolean) => void;
	open: boolean;
	schemaMap: Map<string, ConfigSchemaItem>;
}) {
	const { t } = useTranslation();

	return (
		<section className="overflow-hidden rounded-lg border border-border/60 bg-background">
			<Button
				type="button"
				variant="ghost"
				className="flex h-auto w-full items-center justify-between gap-3 rounded-none px-3 py-2.5 text-left"
				aria-expanded={open}
				onClick={() => onToggle(!open)}
			>
				<span className="min-w-0">
					<span className="block text-sm font-medium">
						{formatMailTemplateGroupLabel(group.templateCode, t)}
					</span>
					{changedCount > 0 ? (
						<span className="mt-0.5 block text-xs font-medium text-primary">
							{t("settings_save_notice", { count: changedCount })}
						</span>
					) : null}
				</span>
				<span className="shrink-0 text-xs text-muted-foreground">
					{open ? t("settings_section_collapse") : t("settings_section_expand")}
				</span>
			</Button>
			<AnimatedCollapsible
				open={open}
				contentClassName={cn(
					"px-3 transition-colors duration-[180ms] ease-out motion-reduce:transition-none",
					open ? "border-t border-border/40" : "border-t border-transparent",
				)}
			>
				<div className="divide-y divide-border/40">
					{group.configs.map((config) => (
						<SettingRow
							key={config.key}
							config={config}
							draft={drafts[config.key] ?? configToDraft(config)}
							schema={schemaMap.get(config.key)}
							templateVariableAction={
								config.key.endsWith("_html")
									? {
											disabled: false,
											onClick: () =>
												onOpenTemplateVariables(group.templateCode),
										}
									: undefined
							}
							onChange={(draft) => onChange(config.key, draft)}
						/>
					))}
				</div>
			</AnimatedCollapsible>
		</section>
	);
}

function getSettingsGroupAction({
	category,
	onOpenTestEmail,
	onRotateYggdrasilSignatureKey,
	rotatingYggdrasilKey,
	t,
}: {
	category: string;
	onOpenTestEmail: () => void;
	onRotateYggdrasilSignatureKey: () => void;
	rotatingYggdrasilKey: boolean;
	t: TFunction<"translation", undefined>;
}) {
	if (category === "mail.config") {
		return (
			<div className="flex flex-col items-start gap-2 lg:items-end">
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={onOpenTestEmail}
				>
					{t("mail_send_test_email")}
				</Button>
				<p className="max-w-xs text-xs text-muted-foreground lg:text-right">
					{t("mail_send_test_email_hint")}
				</p>
			</div>
		);
	}

	if (category === "yggdrasil.signing") {
		return (
			<div className="flex flex-col items-start gap-2 lg:items-end">
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={rotatingYggdrasilKey}
					onClick={onRotateYggdrasilSignatureKey}
				>
					{rotatingYggdrasilKey
						? t("yggdrasil_rotate_signature_key_running")
						: t("yggdrasil_rotate_signature_key")}
				</Button>
				<p className="max-w-xs text-xs text-muted-foreground lg:text-right">
					{t("yggdrasil_rotate_signature_key_hint")}
				</p>
			</div>
		);
	}

	return null;
}

function SettingsSkeleton() {
	return (
		<div className="grid gap-4">
			{Array.from(
				{ length: 4 },
				(_, index) => `settings-skeleton-${index}`,
			).map((key) => (
				<AdminSurface key={key}>
					<div className="h-4 w-40 rounded bg-muted" />
					<div className="mt-4 grid gap-3">
						<div className="h-8 rounded bg-muted/70" />
						<div className="h-8 rounded bg-muted/70" />
					</div>
				</AdminSurface>
			))}
		</div>
	);
}

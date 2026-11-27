import type { ReactNode } from "react";
import { Icon, type IconName } from "@/components/ui/icon";
import { cn } from "@/lib/utils";

export type SettingsSectionId =
	| "profile"
	| "security"
	| "passkeys"
	| "external-auth";

export type SectionDefinition = {
	id: SettingsSectionId;
	icon: IconName;
	label: string;
	description: string;
};

export const SETTINGS_SECTION_IDS: SettingsSectionId[] = [
	"profile",
	"security",
	"passkeys",
	"external-auth",
];

export function settingSectionElementId(id: SettingsSectionId) {
	return `settings-${id}`;
}

export function scrollToSettingsSection(id: SettingsSectionId) {
	const element = document.getElementById(settingSectionElementId(id));
	if (!element) return;
	window.history.replaceState(null, "", `#${id}`);
	element.scrollIntoView({ behavior: "smooth", block: "start" });
}

export function SectionBlock({
	id,
	icon,
	title,
	description,
	children,
}: {
	id: SettingsSectionId;
	icon: IconName;
	title: string;
	description: string;
	children: ReactNode;
}) {
	return (
		<section
			id={settingSectionElementId(id)}
			data-settings-section={id}
			className="scroll-mt-24 border-b border-border/70 py-7 first:pt-0 last:border-b-0 dark:border-white/10"
		>
			<div className="grid gap-5 @min-[42rem]/settings:grid-cols-[14rem_minmax(0,1fr)]">
				<div className="min-w-0">
					<div className="flex items-center gap-2">
						<span className="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary dark:bg-primary/15">
							<Icon name={icon} className="size-4" />
						</span>
						<h2 className="text-base font-semibold text-foreground">{title}</h2>
					</div>
					<p className="mt-2 text-sm leading-6 text-muted-foreground">
						{description}
					</p>
				</div>
				<div className="min-w-0">{children}</div>
			</div>
		</section>
	);
}

export function DetailRow({
	label,
	value,
	action,
}: {
	label: string;
	value: ReactNode;
	action?: ReactNode;
}) {
	return (
		<div className="grid min-h-16 gap-1 rounded-md border border-border/60 bg-muted/18 px-3 py-2.5 dark:border-white/10 dark:bg-muted/10">
			<div className="text-xs font-medium text-muted-foreground">{label}</div>
			<div className="min-w-0 text-sm font-semibold text-foreground">
				{value}
			</div>
			{action ? <div className="pt-1">{action}</div> : null}
		</div>
	);
}

export function SectionNav({
	sections,
	activeSection,
	onSelect,
}: {
	sections: SectionDefinition[];
	activeSection: SettingsSectionId;
	onSelect: (id: SettingsSectionId) => void;
}) {
	return (
		<nav className="space-y-1">
			{sections.map((section) => {
				const active = activeSection === section.id;
				return (
					<button
						key={section.id}
						type="button"
						onClick={() => onSelect(section.id)}
						className={cn(
							"flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors",
							active
								? "bg-accent text-accent-foreground"
								: "text-muted-foreground hover:bg-accent/45 hover:text-foreground",
						)}
					>
						<Icon name={section.icon} className="size-4 shrink-0" />
						<span className="min-w-0 truncate">{section.label}</span>
					</button>
				);
			})}
		</nav>
	);
}

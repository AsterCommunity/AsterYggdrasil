import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import { adminSettingsCategoryPath } from "@/routes/routePaths";
import type { SystemConfig } from "@/types/api";
import { categoryMeta, rootCategory } from "./settingsCategories";

export function SettingsCategoryNav({
	active,
	categories,
	configs,
}: {
	active: string;
	categories: readonly string[];
	configs: SystemConfig[];
}) {
	const { t } = useTranslation();

	return (
		<aside className="min-w-0 xl:sticky xl:top-20 xl:self-start">
			<AdminSurface padded={false} className="overflow-hidden">
				<div className="border-b border-border/70 px-4 py-3 dark:border-white/10">
					<div className="text-sm font-semibold">
						{t("settings_navigation")}
					</div>
					<div className="mt-1 text-xs text-muted-foreground">
						{t("settings_navigation_desc")}
					</div>
				</div>
				<nav className="grid gap-1 p-2">
					{categories.map((category) => (
						<CategoryButton
							key={category}
							active={category === active}
							category={category}
							count={
								configs.filter(
									(config) => rootCategory(config.category) === category,
								).length
							}
						/>
					))}
				</nav>
			</AdminSurface>
		</aside>
	);
}

function CategoryButton({
	active,
	category,
	count,
}: {
	active: boolean;
	category: string;
	count: number;
}) {
	const { t } = useTranslation();
	const meta = categoryMeta[category] ?? {
		descriptionKey: "settings_category_other_desc",
		icon: "Grid" satisfies IconName,
		id: category,
		labelKey: "settings_category_other",
	};
	return (
		<Button
			render={<Link to={adminSettingsCategoryPath(category)} />}
			variant={active ? "default" : "ghost"}
			aria-current={active ? "page" : undefined}
			className={cn(
				"h-auto w-full justify-start whitespace-normal px-3 py-2.5 text-left",
				active
					? "bg-emerald-600 text-white dark:bg-emerald-500/18 dark:text-emerald-100"
					: "text-muted-foreground",
			)}
		>
			<Icon name={meta.icon} className="size-4 shrink-0" aria-hidden="true" />
			<span className="min-w-0 flex-1">
				<span className="block truncate text-sm font-semibold">
					{t(meta.labelKey)}
				</span>
				<span className="mt-0.5 block text-xs opacity-75">
					{t(meta.descriptionKey)}
				</span>
			</span>
			<span className="rounded-full bg-background/80 px-1.5 py-0.5 text-[11px] font-semibold text-foreground dark:bg-white/12 dark:text-current">
				{count}
			</span>
		</Button>
	);
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Icon, type IconName } from "@/components/ui/icon";
import { LauncherSetupCard } from "@/components/yggdrasil/LauncherSetupCard";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	formatAuditDetail,
	formatAuditSummary,
	formatAuditTarget,
} from "@/lib/audit";
import { accountPaths, adminPaths } from "@/routes/routePaths";
import { accountService } from "@/services/accountService";
import { useAuthStore } from "@/stores/authStore";
import type { AccountOverview, AuditLogEntry, AuthUserInfo } from "@/types/api";

const actionItems = [
	{
		to: accountPaths.profiles,
		titleKey: "account.actionProfilesTitle",
		descriptionKey: "account.actionProfilesDescription",
		icon: "User",
	},
	{
		to: accountPaths.wardrobe,
		titleKey: "account.actionWardrobeTitle",
		descriptionKey: "account.actionWardrobeDescription",
		icon: "FileImage",
	},
	{
		to: `${accountPaths.settings}#security`,
		titleKey: "account.actionSessionsTitle",
		descriptionKey: "account.actionSessionsDescription",
		icon: "Key",
	},
	{
		to: adminPaths.settings,
		titleKey: "account.actionAdminTitle",
		descriptionKey: "account.actionAdminDescription",
		icon: "Shield",
		adminOnly: true,
	},
] as const;

const workflowItems = [
	{
		id: "profile",
		titleKey: "account.workflowProfileTitle",
		descriptionKey: "account.workflowProfileDescription",
		icon: "User",
		to: accountPaths.profiles,
	},
	{
		id: "texture",
		titleKey: "account.workflowTextureTitle",
		descriptionKey: "account.workflowTextureDescription",
		icon: "FileImage",
		to: accountPaths.wardrobe,
	},
	{
		id: "launcher",
		titleKey: "account.workflowLauncherTitle",
		descriptionKey: "account.workflowLauncherDescription",
		icon: "Monitor",
		to: accountPaths.profiles,
	},
] as const;

export default function AccountOverviewPage() {
	const { t } = useTranslation();
	const user = useAuthStore((state) => state.user);
	const isAdmin = useAuthStore((state) => state.isAdmin);
	const [overview, setOverview] = useState<AccountOverview | null>(null);
	const accountName = displayNameForAccountUser(user);

	usePageTitle(
		t("account.title", {
			username: accountName,
		}),
	);

	useEffect(() => {
		void accountService
			.overview()
			.then(setOverview)
			.catch(() => undefined);
	}, []);

	const visibleActions = actionItems.filter(
		(item) => !("adminOnly" in item) || !item.adminOnly || isAdmin,
	);
	const recentActivity = overview?.recent_activity ?? [];

	return (
		<div className="min-h-[calc(100dvh-4rem)]">
			<div className="mx-auto grid w-full max-w-[96rem] gap-5 px-4 py-5 sm:px-6 lg:grid-cols-[minmax(0,1fr)_22rem] lg:px-7">
				<div className="contents lg:block lg:min-w-0 lg:space-y-5">
					<AccountHero username={accountName} />

					<WorkflowPanel />

					<LauncherSetupCard
						className="order-4 border-border/70 bg-card/86 shadow-sm backdrop-blur dark:border-white/10 dark:bg-card/72 dark:shadow-none lg:order-none"
						showServerOwner
					/>
				</div>

				<aside className="contents lg:sticky lg:top-20 lg:block lg:self-start lg:space-y-5">
					<QuickActionsPanel items={visibleActions} />
					<RecentActivityPanel items={recentActivity} />
				</aside>
			</div>
		</div>
	);
}

function displayNameForAccountUser(user: AuthUserInfo | null) {
	return (
		user?.profile?.display_name?.trim() || user?.username?.trim() || "Player"
	);
}

function AccountHero({ username }: { username: string }) {
	const { t } = useTranslation();

	return (
		<section className="relative order-1 overflow-hidden rounded-xl border border-border/70 bg-card text-card-foreground shadow-sm dark:border-white/10 dark:bg-card/78 dark:shadow-none lg:order-none">
			<div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/40 to-transparent" />
			<div
				className="absolute inset-y-0 right-0 w-[78%] bg-cover bg-center opacity-58 sm:w-[70%] sm:opacity-70 md:w-[62%] md:opacity-82 dark:opacity-26 sm:dark:opacity-32 md:dark:opacity-36"
				style={{ backgroundImage: "url('/static/images/home.webp')" }}
			/>
			<div className="absolute inset-0 bg-[linear-gradient(90deg,var(--card)_0%,var(--card)_42%,color-mix(in_oklch,var(--card)_86%,transparent)_66%,color-mix(in_oklch,var(--card)_38%,transparent)_100%)] sm:bg-[linear-gradient(90deg,var(--card)_0%,var(--card)_38%,color-mix(in_oklch,var(--card)_88%,transparent)_58%,color-mix(in_oklch,var(--card)_42%,transparent)_100%)] md:bg-[linear-gradient(90deg,var(--card)_0%,var(--card)_46%,color-mix(in_oklch,var(--card)_92%,transparent)_62%,color-mix(in_oklch,var(--card)_58%,transparent)_82%,color-mix(in_oklch,var(--card)_22%,transparent)_100%)]" />
			<div className="absolute inset-0 bg-[radial-gradient(circle_at_78%_0%,color-mix(in_oklch,var(--primary)_18%,transparent),transparent_28rem)] opacity-65 dark:opacity-42" />
			<div className="relative p-6 sm:p-7">
				<div className="min-w-0">
					<h1 className="max-w-3xl text-3xl font-semibold leading-tight tracking-normal sm:text-4xl">
						{t("account.welcome", { username })}
					</h1>
					<p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground sm:text-base">
						{t("account.description")}
					</p>
				</div>
			</div>
		</section>
	);
}

function WorkflowPanel() {
	const { t } = useTranslation();

	return (
		<section className="order-2 hidden rounded-xl border border-border/70 bg-card/64 p-3 shadow-xs backdrop-blur dark:border-white/10 dark:bg-card/48 dark:shadow-none lg:order-none lg:block">
			<div className="flex items-center justify-between gap-3 px-1 pb-3">
				<div className="min-w-0">
					<h2 className="text-sm font-semibold">{t("account.nextActions")}</h2>
					<p className="mt-0.5 text-xs text-muted-foreground">
						{t("account.nextActionsDescription")}
					</p>
				</div>
				<div className="hidden h-px flex-1 bg-border/70 xl:block" />
			</div>
			<div className="grid gap-2 xl:grid-cols-3">
				{workflowItems.map((item, index) => (
					<WorkflowCard
						key={item.id}
						description={t(item.descriptionKey)}
						icon={item.icon}
						index={index + 1}
						to={item.to}
						title={t(item.titleKey)}
					/>
				))}
			</div>
		</section>
	);
}

function WorkflowCard({
	description,
	icon,
	index,
	title,
	to,
}: {
	description: string;
	icon: IconName;
	index: number;
	title: string;
	to: string;
}) {
	return (
		<Link
			to={to}
			className="group grid min-h-28 grid-rows-[auto_minmax(0,1fr)] gap-3 rounded-lg border border-transparent bg-background/50 p-3 transition-[border-color,background-color] hover:border-primary/28 hover:bg-accent/38 dark:bg-background/20 dark:hover:bg-accent/16"
		>
			<div className="flex items-center justify-between gap-3">
				<div className="flex min-w-0 items-center gap-2.5">
					<Icon name={icon} className="size-5 shrink-0 text-primary" />
					<div className="truncate text-sm font-semibold">{title}</div>
				</div>
				<div className="font-mono text-[11px] font-semibold text-muted-foreground">
					{String(index).padStart(2, "0")}
				</div>
			</div>
			<div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-end gap-2">
				<p className="line-clamp-2 text-xs leading-5 text-muted-foreground">
					{description}
				</p>
				<Icon
					name="ArrowRight"
					className="size-4 text-primary transition-transform group-hover:translate-x-0.5"
				/>
			</div>
		</Link>
	);
}

function RecentActivityPanel({ items }: { items: AuditLogEntry[] }) {
	const { t } = useTranslation();

	return (
		<section className="order-3 rounded-xl border border-border/70 bg-card/74 p-5 shadow-xs backdrop-blur dark:border-white/10 dark:bg-card/64 dark:shadow-none lg:order-none">
			<div className="flex items-start justify-between gap-3">
				<div>
					<div className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
						{t("account.recentActivityEyebrow")}
					</div>
					<h2 className="mt-1 text-lg font-semibold">
						{t("account.recentActivityTitle")}
					</h2>
				</div>
				<span className="mt-1 size-2.5 rounded-full bg-primary/80 shadow-[0_0_0_5px_color-mix(in_oklch,var(--primary)_16%,transparent)]" />
			</div>
			{items.length > 0 ? (
				<div className="mt-5 divide-y divide-border/60 dark:divide-white/10">
					{items.map((item) => {
						const detail = formatAuditDetail(t, item);
						return (
							<div key={item.id} className="py-3 first:pt-0 last:pb-0">
								<div className="flex items-start gap-3">
									<span className="mt-1.5 size-2 rounded-full bg-primary/75" />
									<div className="min-w-0 flex-1">
										<div className="truncate text-sm font-semibold">
											{formatAuditSummary(t, item)}
										</div>
										<div className="mt-1 truncate text-xs text-muted-foreground">
											{formatAuditTarget(t, item)}
										</div>
										{detail ? (
											<div className="mt-1 truncate text-xs text-muted-foreground">
												{detail}
											</div>
										) : null}
									</div>
								</div>
								<time className="mt-2 block pl-5 text-xs text-muted-foreground">
									{new Date(item.created_at).toLocaleString()}
								</time>
							</div>
						);
					})}
				</div>
			) : (
				<div className="mt-5 rounded-lg border border-dashed border-border/70 p-4 text-sm leading-6 text-muted-foreground dark:border-white/12">
					{t("account.recentActivityEmpty")}
				</div>
			)}
		</section>
	);
}

function QuickActionsPanel({
	items,
}: {
	items: readonly (typeof actionItems)[number][];
}) {
	const { t } = useTranslation();

	return (
		<section className="order-2 rounded-xl border border-border/70 bg-card/82 p-5 shadow-sm backdrop-blur dark:border-white/10 dark:bg-card/72 dark:shadow-none lg:order-none">
			<h2 className="text-lg font-semibold">{t("account.quickActions")}</h2>
			<div className="mt-4 grid gap-3">
				{items.map((item) => (
					<Link
						key={item.to}
						to={item.to}
						className="group grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 rounded-lg border border-border/60 bg-background/55 p-3 transition-[border-color,background-color] hover:border-primary/35 hover:bg-accent/45 dark:border-white/10 dark:bg-background/28 dark:hover:border-primary/35 dark:hover:bg-accent/20"
					>
						<Icon name={item.icon} className="size-5 text-primary" />
						<div className="min-w-0">
							<div className="truncate font-semibold">{t(item.titleKey)}</div>
							<div className="mt-0.5 truncate text-xs text-muted-foreground">
								{t(item.descriptionKey)}
							</div>
						</div>
						<Icon
							name="ArrowRight"
							className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary"
						/>
					</Link>
				))}
			</div>
		</section>
	);
}

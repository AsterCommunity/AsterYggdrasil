import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { buttonVariants } from "@/components/ui/buttonVariants";
import { Icon, type IconName } from "@/components/ui/icon";
import { usePageTitle } from "@/hooks/usePageTitle";
import { cn } from "@/lib/utils";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type RouteAccessStateProps = {
	actionHref?: string;
	actionLabelKey: string;
	actionOnClick?: () => void;
	descriptionKey: string;
	icon: IconName;
	titleKey: string;
};

export function RouteAccessState({
	actionHref,
	actionLabelKey,
	actionOnClick,
	descriptionKey,
	icon,
	titleKey,
}: RouteAccessStateProps) {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const brandTitle = branding.title || "AsterYggdrasil";
	const title = t(titleKey);

	usePageTitle(title);

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={t("brand.tagline")}
			variant="auth"
		>
			<main className="mx-auto flex w-full max-w-2xl flex-1 items-center px-4 py-12 sm:px-6">
				<section className="grid w-full gap-5 rounded-2xl border border-border/70 bg-card/88 p-6 text-card-foreground shadow-2xl shadow-black/10 ring-1 ring-foreground/5 backdrop-blur-xl sm:p-8">
					<div className="flex size-12 items-center justify-center rounded-xl bg-muted text-muted-foreground">
						<Icon name={icon} className="size-6" />
					</div>
					<div className="grid gap-2">
						<h1 className="text-2xl font-semibold tracking-normal">{title}</h1>
						<p className="text-sm leading-6 text-muted-foreground">
							{t(descriptionKey)}
						</p>
					</div>
					{actionHref ? (
						<Link
							to={actionHref}
							className={cn(buttonVariants({ size: "sm" }), "w-fit")}
						>
							{t(actionLabelKey)}
						</Link>
					) : (
						<button
							type="button"
							onClick={actionOnClick}
							className={cn(buttonVariants({ size: "sm" }), "w-fit")}
						>
							{t(actionLabelKey)}
						</button>
					)}
				</section>
			</main>
		</PublicEntryShell>
	);
}

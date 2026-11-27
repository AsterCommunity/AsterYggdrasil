import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { buttonVariants } from "@/components/ui/buttonVariants";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import { publicPaths } from "@/routes/routePaths";

export function TextureLibraryDisabledPanel() {
	const { t } = useTranslation();
	return (
		<section className="grid min-h-[22rem] place-items-center rounded-xl border border-black/10 bg-white/76 px-4 text-center shadow-2xl shadow-emerald-950/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/25">
			<div className="max-w-md">
				<div className="mx-auto flex size-11 items-center justify-center rounded-xl border border-emerald-700/16 bg-emerald-600/10 text-emerald-700 dark:border-emerald-300/18 dark:bg-emerald-400/12 dark:text-emerald-200">
					<Icon name="Images" className="size-5" />
				</div>
				<h1 className="mt-4 text-xl font-semibold tracking-normal">
					{t("library.disabledTitle")}
				</h1>
				<p className="mt-2 text-sm leading-6 text-muted-foreground">
					{t("library.disabledDescription")}
				</p>
				<Link
					to={publicPaths.home}
					className={cn(buttonVariants({ variant: "outline" }), "mt-5")}
				>
					{t("errorPage.backHome")}
				</Link>
			</div>
		</section>
	);
}

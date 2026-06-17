import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { BrandMark } from "@/components/layout/BrandMark";
import { Icon } from "@/components/ui/icon";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

const footerLinks = [
	{ id: "account", to: accountPaths.home, label: "nav.account" },
	{ id: "tos", to: publicPaths.tos, label: "nav.tos" },
	{ id: "privacy", to: publicPaths.privacy, label: "nav.privacy" },
] as const;

function scrollToPageTop() {
	if (typeof window === "undefined") return;
	window.scrollTo({ top: 0, left: 0, behavior: "auto" });
}

export function AppFooter() {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const title = branding.title || t("brand.name");
	const description = branding.description || t("footer.description");
	const year = new Date().getFullYear();

	return (
		<footer className="min-w-0 overflow-x-clip border-black/10 border-t bg-white/72 text-slate-700 backdrop-blur-xl dark:border-white/10 dark:bg-[#050b09] dark:text-slate-300">
			<div className="mx-auto grid w-full min-w-0 max-w-7xl gap-8 px-4 py-10 sm:px-6 md:grid-cols-[minmax(0,1fr)_280px]">
				<div className="min-w-0 max-w-xl">
					<Link
						to={publicPaths.home}
						className="flex max-w-full min-w-0 items-center gap-3"
						onClick={scrollToPageTop}
					>
						<BrandMark
							branding={branding}
							className="size-9 shrink-0 object-contain"
						/>
						<span className="min-w-0">
							<span className="block truncate font-semibold text-[#102118] dark:text-white">
								{title}
							</span>
							<span className="block truncate font-medium text-[0.68rem] text-emerald-700 uppercase tracking-[0.18em] dark:text-emerald-300">
								{t("brand.tagline")}
							</span>
						</span>
					</Link>
					<p className="mt-4 max-w-md text-sm leading-6 text-slate-600 dark:text-slate-400">
						{description}
					</p>
				</div>

				<nav aria-label={t("footer.navigation")} className="grid gap-3">
					<div className="text-xs font-semibold text-slate-500 uppercase tracking-[0.16em] dark:text-slate-500">
						{t("footer.navigation")}
					</div>
					<div className="grid gap-2">
						{footerLinks.map((link) => (
							<Link
								key={link.id}
								to={link.to}
								onClick={scrollToPageTop}
								className="inline-flex items-center gap-2 text-sm text-slate-700 transition-colors hover:text-emerald-700 dark:text-slate-300 dark:hover:text-emerald-300"
							>
								<Icon name="ArrowRight" className="size-3.5" />
								{t(link.label)}
							</Link>
						))}
					</div>
				</nav>
			</div>
			<div className="min-w-0 border-black/10 border-t px-4 py-4 text-center text-xs text-slate-500 dark:border-white/10 dark:text-slate-500">
				{t("footer.copyright", { year, name: title })}
			</div>
		</footer>
	);
}

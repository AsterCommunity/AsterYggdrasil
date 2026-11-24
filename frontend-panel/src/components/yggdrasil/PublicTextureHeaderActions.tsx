import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import { buttonVariants } from "@/components/ui/buttonVariants";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import type { AuthUserInfo } from "@/types/api";

type Props = {
	isAuthenticated: boolean;
	user?: AuthUserInfo | null;
	onLogout: () => void;
};

export function PublicTextureHeaderActions({
	isAuthenticated,
	user,
	onLogout,
}: Props) {
	const { t } = useTranslation();

	if (!isAuthenticated || !user) {
		return (
			<Link
				to={publicPaths.login}
				className={cn(
					buttonVariants({ variant: "default", size: "sm" }),
					"h-10 rounded-full border-emerald-300/24 bg-emerald-500 px-3 text-white shadow-lg shadow-emerald-950/35 hover:bg-emerald-400 sm:px-4",
				)}
			>
				<Icon name="SignIn" className="size-4" />
				<span className="hidden sm:inline">{t("home.loginRegister")}</span>
			</Link>
		);
	}

	const workspaceLabel = t("nav.backToAccount");

	return (
		<>
			<AuthUserMenu
				user={user}
				scope="public"
				showAccountLink={false}
				className="border-black/10 bg-white/64 text-[#102118] shadow-lg shadow-black/12 backdrop-blur hover:bg-white/80 aria-expanded:bg-white/80 dark:border-white/14 dark:bg-white/8 dark:text-white dark:shadow-black/20 dark:hover:bg-white/14 dark:aria-expanded:bg-white/14"
				onLogout={onLogout}
			/>
			<Link
				to={accountPaths.home}
				aria-label={workspaceLabel}
				title={workspaceLabel}
				className={cn(
					buttonVariants({ variant: "outline", size: "sm" }),
					"h-10 rounded-full border-black/10 bg-white/64 px-2 text-[#102118] shadow-lg shadow-black/12 backdrop-blur hover:bg-white/80 dark:border-white/14 dark:bg-white/8 dark:text-white dark:shadow-black/20 dark:hover:bg-white/14 sm:px-3",
				)}
			>
				<Icon name="ArrowLeft" className="size-4" />
				<span className="hidden sm:inline">{workspaceLabel}</span>
			</Link>
		</>
	);
}

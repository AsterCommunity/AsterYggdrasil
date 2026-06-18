import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Icon } from "@/components/ui/icon";
import { firstAdminPathForScopes } from "@/lib/operatorScopes";
import { cn } from "@/lib/utils";
import { accountPaths, adminPaths, publicPaths } from "@/routes/routePaths";
import type { AuthUserInfo } from "@/types/api";

type AuthUserMenuProps = {
	user: AuthUserInfo;
	scope?: "public" | "account" | "admin";
	onLogout: () => void;
	className?: string;
};

export function AuthUserMenu({
	user,
	scope = "account",
	onLogout,
	className,
}: AuthUserMenuProps) {
	const { t } = useTranslation();
	const userName = user.username.trim() || "User";
	const displayName = user.profile?.display_name?.trim() || userName;
	const avatar = user.profile?.avatar;
	const isAdmin = user.role === "admin";
	const firstOperatorAdminPath =
		user.role === "operator"
			? firstAdminPathForScopes(user.operator_scopes ?? [])
			: null;
	const adminEntryPath = isAdmin ? adminPaths.settings : firstOperatorAdminPath;
	const isAdminScope = scope === "admin";
	const isAppScope = scope !== "public";

	return (
		<DropdownMenu>
			<DropdownMenuTrigger
				render={
					<Button
						type="button"
						variant="ghost"
						size="sm"
						className={cn(
							"h-10 min-w-0 max-w-60 gap-2 rounded-full border border-border/60 px-1.5 pr-1.5 shadow-xs hover:bg-card aria-expanded:bg-card sm:pr-2.5",
							isAdminScope ? "bg-card/65" : "bg-card/70",
							className,
						)}
						aria-label={displayName}
					/>
				}
			>
				<UserAvatarImage
					name={displayName}
					avatar={avatar}
					size="sm"
					className="rounded-xl bg-muted/80 text-muted-foreground ring-border/60"
				/>
				<span className="hidden min-w-0 max-w-28 truncate text-sm font-semibold sm:block">
					{displayName}
				</span>
				{isAdmin || firstOperatorAdminPath ? (
					<Badge
						variant="outline"
						className="hidden border-border/50 bg-background/70 px-1.5 py-0 text-[11px] text-muted-foreground md:inline-flex"
					>
						{isAdmin ? "admin" : "operator"}
					</Badge>
				) : null}
				<Icon
					name="CaretDown"
					className="hidden size-3.5 shrink-0 text-muted-foreground sm:block"
				/>
			</DropdownMenuTrigger>
			<DropdownMenuContent
				align="end"
				className="w-64 border-border/70 bg-popover/95 p-2 text-popover-foreground shadow-2xl shadow-black/25 backdrop-blur-xl"
			>
				<div className="flex items-center gap-3 rounded-md bg-muted/35 px-3 py-2">
					<UserAvatarImage
						name={displayName}
						avatar={avatar}
						size="md"
						className="rounded-xl bg-muted/70 text-muted-foreground ring-border/60"
					/>
					<div className="min-w-0">
						<div className="truncate text-sm font-semibold text-popover-foreground">
							{displayName}
						</div>
						<div className="mt-0.5 text-xs text-muted-foreground">
							<span className="truncate">
								{userName}
								{user.role ? ` · ${user.role}` : ""}
							</span>
						</div>
					</div>
				</div>
				{scope === "public" ? (
					<div className="mt-2 grid gap-1">
						<DropdownMenuItem
							render={<Link to={accountPaths.home} />}
							className="flex min-h-9 items-center gap-2 rounded-md px-3 py-2 text-sm text-popover-foreground transition-colors hover:bg-accent focus:bg-accent"
						>
							<Icon name="Gauge" className="size-4 text-muted-foreground" />
							{t("nav.account")}
						</DropdownMenuItem>
						{adminEntryPath ? (
							<DropdownMenuItem
								render={<Link to={adminEntryPath} />}
								className="flex min-h-9 items-center gap-2 rounded-md px-3 py-2 text-sm text-popover-foreground transition-colors hover:bg-accent focus:bg-accent"
							>
								<Icon name="Shield" className="size-4 text-muted-foreground" />
								{t("nav.admin")}
							</DropdownMenuItem>
						) : null}
					</div>
				) : isAppScope ? (
					<div className="mt-2 grid gap-1">
						<DropdownMenuItem
							render={<Link to={publicPaths.home} />}
							className="flex min-h-9 items-center gap-2 rounded-md px-3 py-2 text-sm text-popover-foreground transition-colors hover:bg-accent focus:bg-accent"
						>
							<Icon name="House" className="size-4 text-muted-foreground" />
							{t("common.backToHome")}
						</DropdownMenuItem>
						{isAdminScope ? (
							<DropdownMenuItem
								render={<Link to={accountPaths.settings} />}
								className="flex min-h-9 items-center gap-2 rounded-md px-3 py-2 text-sm text-popover-foreground transition-colors hover:bg-accent focus:bg-accent"
							>
								<Icon name="Gear" className="size-4 text-muted-foreground" />
								{t("nav.personalSettings")}
							</DropdownMenuItem>
						) : null}
					</div>
				) : null}
				<DropdownMenuItem
					render={<button type="button" />}
					variant="destructive"
					className="mt-2 flex min-h-9 w-full items-center justify-start gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-destructive/10 focus:bg-destructive/10"
					onClick={onLogout}
				>
					<Icon name="SignOut" className="size-4" />
					{t("nav.logout")}
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

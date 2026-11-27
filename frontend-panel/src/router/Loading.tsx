import { useTranslation } from "react-i18next";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";

type LoadingProps = {
	surface?: "default" | "public";
};

export function Loading({ surface = "default" }: LoadingProps) {
	const { t } = useTranslation();
	return (
		<div
			className={cn(
				"flex min-h-dvh items-center justify-center",
				surface === "public"
					? "bg-public-surface text-public-foreground"
					: "bg-background text-muted-foreground",
			)}
		>
			<Icon
				name="Spinner"
				className={cn(
					"size-5 animate-spin",
					surface === "public"
						? "text-emerald-700 dark:text-emerald-200"
						: "text-muted-foreground",
				)}
				aria-hidden="true"
			/>
			<span className="sr-only">{t("common.loading")}</span>
		</div>
	);
}

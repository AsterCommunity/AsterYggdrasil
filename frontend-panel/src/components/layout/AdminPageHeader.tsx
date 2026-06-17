import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function AdminPageHeader({
	actions,
	className,
	description,
	title,
	toolbar,
}: {
	actions?: ReactNode;
	className?: string;
	description?: string;
	title: string;
	toolbar?: ReactNode;
}) {
	return (
		<header
			className={cn(
				"border-b border-border/70 pb-5 dark:border-white/10",
				className,
			)}
		>
			<div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
				<div className="min-w-0">
					<h1 className="text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
						{title}
					</h1>
					{description ? (
						<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{actions ? (
					<div className="flex shrink-0 flex-wrap items-center gap-2 md:justify-end">
						{actions}
					</div>
				) : null}
			</div>
			{toolbar ? (
				<div className="mt-4 flex flex-wrap items-center gap-2">{toolbar}</div>
			) : null}
		</header>
	);
}

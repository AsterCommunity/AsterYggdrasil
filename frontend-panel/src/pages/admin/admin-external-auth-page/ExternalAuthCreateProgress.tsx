import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ExternalAuthCreateStep } from "./shared";

export function ExternalAuthCreateProgress({
	createStep,
	createSteps,
	onCreateStepChange,
}: {
	createStep: number;
	createSteps: ExternalAuthCreateStep[];
	onCreateStepChange: (step: number) => void;
}) {
	const { t } = useTranslation();
	const currentStep = createSteps[Math.min(createStep, createSteps.length - 1)];

	return (
		<div className="space-y-3">
			<div className="rounded-lg border border-border/70 bg-muted/20 p-4">
				<div className="flex items-start justify-between gap-4">
					<div className="min-w-0 space-y-1">
						<p className="text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
							{t("admin.externalAuth.progress", {
								current: createStep + 1,
								total: createSteps.length,
							})}
						</p>
						<h3 className="text-base font-semibold">{currentStep.title}</h3>
						<p className="text-sm leading-6 text-muted-foreground">
							{currentStep.description}
						</p>
					</div>
					<div className="hidden text-3xl font-semibold text-foreground/15 sm:block">
						{String(createStep + 1).padStart(2, "0")}
					</div>
				</div>
				<div className="mt-4 h-1.5 overflow-hidden rounded-full bg-background">
					<div
						className="h-full rounded-full bg-primary transition-[width] duration-300"
						style={{
							width: `${((createStep + 1) / createSteps.length) * 100}%`,
						}}
					/>
				</div>
			</div>
			<div className="hidden grid-cols-3 gap-2 md:grid">
				{createSteps.map((step, index) => (
					<button
						key={step.title}
						type="button"
						disabled={index > createStep}
						onClick={() => onCreateStepChange(index)}
						className={cn(
							"flex items-center gap-2 rounded-lg border p-3 text-left text-sm transition",
							index === createStep
								? "border-primary/45 bg-primary/8 text-foreground"
								: index < createStep
									? "border-border/80 bg-background hover:border-primary/35"
									: "border-border/60 bg-muted/20 text-muted-foreground",
						)}
					>
						<span className="grid size-6 shrink-0 place-items-center rounded-md border border-border/70 bg-background text-[11px] font-semibold">
							{index + 1}
						</span>
						<span className="truncate font-medium">{step.title}</span>
					</button>
				))}
			</div>
		</div>
	);
}

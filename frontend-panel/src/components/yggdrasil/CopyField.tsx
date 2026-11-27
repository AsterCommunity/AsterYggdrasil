import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { useClipboard } from "@/hooks/useClipboard";
import { cn } from "@/lib/utils";

type CopyFieldProps = {
	label: string;
	value: string;
	compact?: boolean;
	inputClassName?: string;
};

export function CopyField({
	label,
	value,
	compact = false,
	inputClassName,
}: CopyFieldProps) {
	const { t } = useTranslation();
	const { copied, copy: writeClipboard } = useClipboard();

	return (
		<div className="grid min-w-0 gap-1.5">
			<span className="text-xs font-medium text-muted-foreground">{label}</span>
			<span className="flex min-w-0 max-w-full gap-2">
				<Input
					readOnly
					value={value}
					className={cn(
						compact ? "h-8 font-mono text-xs" : "font-mono text-xs",
						inputClassName,
					)}
				/>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className="shrink-0"
					aria-label={t("common.copy", { label })}
					onClick={() => void writeClipboard(value)}
				>
					<Icon name={copied ? "Check" : "Copy"} className="size-4" />
				</Button>
			</span>
		</div>
	);
}

import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AdminNumberUnitInput } from "@/components/admin/AdminNumberUnitInput";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { convertNumberUnitValueToBaseUnit } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import type { ConfigSchemaItem, SystemConfig } from "@/types/api";
import {
	editorLanguage,
	formatDisplayValue,
	getAvailableDisplayUnits,
	getPreferredDisplayUnit,
	getTimeConfigBaseUnit,
	type TimeDisplayUnitValue,
	timeDisplayUnits,
	translateOrFallback,
} from "./settingsCategories";
import { createDraftArrayRow, type DraftValue } from "./settingsDraft";

export function SettingControl({
	config,
	draft,
	onChange,
	schema,
	controlId,
	labelledById,
}: {
	config: SystemConfig;
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	schema?: ConfigSchemaItem;
	/** id applied to the primary form control (Label htmlFor target). */
	controlId?: string;
	/** id of the row's visible label, referenced by grouped controls. */
	labelledById?: string;
}) {
	const { t } = useTranslation();
	if (config.value_type === "boolean") {
		return (
			<BooleanControl draft={draft} onChange={onChange} controlId={controlId} />
		);
	}
	if (config.value_type === "number") {
		return (
			<NumberControl
				config={config}
				draft={draft}
				onChange={onChange}
				controlId={controlId}
			/>
		);
	}
	if (config.value_type === "string_enum") {
		return (
			<StringEnumControl
				draft={draft}
				options={schema?.options ?? []}
				onChange={onChange}
				controlId={controlId}
				labelledById={labelledById}
			/>
		);
	}
	if (config.value_type === "string_enum_set") {
		return (
			<EnumSetControl
				draft={draft}
				options={schema?.options ?? []}
				onChange={onChange}
				controlId={controlId}
				labelledById={labelledById}
			/>
		);
	}
	if (config.value_type === "string_array") {
		return (
			<StringArrayControl
				draft={draft}
				options={schema?.options ?? []}
				onChange={onChange}
				controlId={controlId}
				labelledById={labelledById}
			/>
		);
	}
	if (config.value_type === "multiline") {
		return (
			<CodeTextControl
				config={config}
				draft={draft}
				language={editorLanguage(config)}
				onChange={onChange}
				controlId={controlId}
			/>
		);
	}
	return (
		<Input
			id={controlId}
			type={config.is_sensitive ? "password" : "text"}
			value={draft.text}
			placeholder={
				config.is_sensitive ? t("settings_sensitive_keep_placeholder") : ""
			}
			onChange={(event) =>
				onChange({ ...draft, text: event.currentTarget.value })
			}
			className={config.is_sensitive ? "font-mono" : undefined}
		/>
	);
}

function BooleanControl({
	draft,
	onChange,
	controlId,
}: {
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	controlId?: string;
}) {
	const { t } = useTranslation();
	const checked = draft.text === "true";
	return (
		<div className="flex items-center gap-3">
			<Switch
				id={controlId}
				checked={checked}
				onCheckedChange={(nextChecked) =>
					onChange({ ...draft, text: nextChecked ? "true" : "false" })
				}
			/>
			<span className="text-sm text-muted-foreground">
				{checked ? t("settings_value_on") : t("settings_value_off")}
			</span>
		</div>
	);
}

function NumberControl({
	config,
	draft,
	onChange,
	controlId,
}: {
	config: SystemConfig;
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	controlId?: string;
}) {
	const { t } = useTranslation();
	const baseUnit = getTimeConfigBaseUnit(config);
	const units = baseUnit ? timeDisplayUnits[baseUnit] : null;
	const [displayUnits, setDisplayUnits] = useState<
		Partial<Record<string, TimeDisplayUnitValue>>
	>({});

	if (!units) {
		return (
			<Input
				id={controlId}
				type="number"
				inputMode="numeric"
				min={0}
				step={1}
				value={draft.text}
				onChange={(event) =>
					onChange({ ...draft, text: event.currentTarget.value })
				}
			/>
		);
	}

	const availableUnits = getAvailableDisplayUnits(units, draft.text);
	const preferredUnit = getPreferredDisplayUnit(units, draft.text);
	const selectedUnit =
		availableUnits.find((unit) => unit.value === displayUnits[config.key]) ??
		preferredUnit;
	const displayValue = formatDisplayValue(draft.text, selectedUnit);

	function updateFromDisplayValue(value: string) {
		const nextDisplayValue = value.trim();
		if (!nextDisplayValue) {
			setDisplayUnits((previous) => ({
				...previous,
				[config.key]: selectedUnit.value,
			}));
			onChange({ ...draft, text: "" });
			return;
		}

		const nextValue = convertNumberUnitValueToBaseUnit(
			nextDisplayValue,
			selectedUnit,
		);
		if (nextValue === null) {
			setDisplayUnits((previous) => ({
				...previous,
				[config.key]: selectedUnit.value,
			}));
			onChange({ ...draft, text: nextDisplayValue });
			return;
		}

		// Pin the unit that produced this value: otherwise the recomputed
		// preferred unit can silently switch (e.g. 86400s renders as "1" day)
		// and the next keystroke would be interpreted in the wrong unit.
		setDisplayUnits((previous) => ({
			...previous,
			[config.key]: selectedUnit.value,
		}));
		onChange({ ...draft, text: String(nextValue) });
	}

	return (
		<AdminNumberUnitInput
			id={controlId}
			value={displayValue}
			unit={selectedUnit.value}
			units={availableUnits}
			placeholder={t("common.value")}
			unitAriaLabel={t("settings_time_unit_label")}
			onValueChange={updateFromDisplayValue}
			onUnitChange={(value) => {
				setDisplayUnits((previous) => ({
					...previous,
					[config.key]: value,
				}));
			}}
		/>
	);
}

function EnumSetControl({
	draft,
	onChange,
	options,
	controlId,
	labelledById,
}: {
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	options: NonNullable<ConfigSchemaItem["options"]>;
	controlId?: string;
	labelledById?: string;
}) {
	const { t } = useTranslation();
	const [filter, setFilter] = useState("");
	const selected = new Set(draft.array);
	const normalizedFilter = filter.trim().toLowerCase();
	const visibleOptions = options.filter((option) =>
		`${option.value} ${translateOrFallback(t, option.label_i18n_key, option.value)}`
			.toLowerCase()
			.includes(normalizedFilter),
	);

	return (
		<fieldset
			id={controlId}
			aria-labelledby={labelledById}
			className="grid gap-2"
		>
			<div className="flex flex-wrap items-center gap-2">
				<Input
					value={filter}
					onChange={(event) => setFilter(event.currentTarget.value)}
					placeholder={t("settings_enum_set_search_placeholder")}
					className="max-w-72"
				/>
				<span className="text-xs text-muted-foreground">
					{t("settings_enum_set_selected_count", {
						count: selected.size,
						total: options.length,
					})}
				</span>
			</div>
			<div className="flex max-h-72 flex-wrap gap-2 overflow-auto rounded-lg border border-border/70 bg-muted/15 p-2 dark:border-white/10">
				{visibleOptions.map((option) => {
					const active = selected.has(option.value);
					return (
						<Button
							key={option.value}
							type="button"
							variant={active ? "default" : "outline"}
							size="xs"
							aria-pressed={active}
							onClick={() => {
								const next = new Set(selected);
								if (active) {
									next.delete(option.value);
								} else {
									next.add(option.value);
								}
								onChange({ ...draft, array: Array.from(next).sort() });
							}}
							className={cn(
								"max-w-full whitespace-normal",
								active
									? "dark:border-emerald-400/40 dark:bg-emerald-400/20 dark:text-emerald-100"
									: "text-muted-foreground",
							)}
						>
							{translateOrFallback(t, option.label_i18n_key, option.value)}
						</Button>
					);
				})}
			</div>
		</fieldset>
	);
}

function StringEnumControl({
	draft,
	onChange,
	options,
	controlId,
	labelledById,
}: {
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	options: NonNullable<ConfigSchemaItem["options"]>;
	controlId?: string;
	labelledById?: string;
}) {
	const { t } = useTranslation();
	const selected = draft.text.trim();

	return (
		<fieldset
			id={controlId}
			aria-labelledby={labelledById}
			className="flex max-h-72 flex-wrap gap-2 overflow-auto rounded-lg border border-border/70 bg-muted/15 p-2 dark:border-white/10"
		>
			{options.map((option) => {
				const active = selected === option.value;
				return (
					<Button
						key={option.value}
						type="button"
						variant={active ? "default" : "outline"}
						size="xs"
						aria-pressed={active}
						onClick={() =>
							onChange({ ...draft, text: option.value, array: [] })
						}
						className={cn(
							"max-w-full whitespace-normal",
							active
								? "dark:border-emerald-400/40 dark:bg-emerald-400/20 dark:text-emerald-100"
								: "text-muted-foreground",
						)}
					>
						{translateOrFallback(t, option.label_i18n_key, option.value)}
					</Button>
				);
			})}
		</fieldset>
	);
}

function StringArrayControl({
	draft,
	onChange,
	options,
	controlId,
	labelledById,
}: {
	draft: DraftValue;
	onChange: (draft: DraftValue) => void;
	options: NonNullable<ConfigSchemaItem["options"]>;
	controlId?: string;
	labelledById?: string;
}) {
	const { t } = useTranslation();
	if (options.length > 0) {
		return (
			<EnumSetControl
				draft={draft}
				options={options}
				onChange={onChange}
				controlId={controlId}
				labelledById={labelledById}
			/>
		);
	}
	return (
		<fieldset
			id={controlId}
			aria-labelledby={labelledById}
			className="grid gap-2"
		>
			{draft.arrayRows.map((row, index) => (
				<div key={row.id} className="flex gap-2">
					<Input
						value={row.value}
						onChange={(event) => {
							const value = event.currentTarget.value;
							const arrayRows = draft.arrayRows.map((item, itemIndex) =>
								itemIndex === index ? { ...item, value } : item,
							);
							onChange({
								...draft,
								array: arrayRows.flatMap((item) => {
									const nextValue = item.value.trim();
									return nextValue ? [nextValue] : [];
								}),
								arrayRows,
							});
						}}
					/>
					<Button
						type="button"
						variant="outline"
						size="icon"
						onClick={() =>
							onChange({
								...draft,
								array: draft.arrayRows.flatMap((item, itemIndex) => {
									if (itemIndex === index) return [];
									const nextValue = item.value.trim();
									return nextValue ? [nextValue] : [];
								}),
								arrayRows: draft.arrayRows.filter(
									(_, itemIndex) => itemIndex !== index,
								),
							})
						}
						aria-label={t("settings_string_array_remove_item")}
					>
						<span aria-hidden="true">x</span>
					</Button>
				</div>
			))}
			<Button
				type="button"
				variant="outline"
				size="sm"
				className="w-fit"
				onClick={() => {
					const arrayRows = [...draft.arrayRows, createDraftArrayRow("")];
					onChange({
						...draft,
						array: arrayRows.map((item) => item.value),
						arrayRows,
					});
				}}
			>
				{t("settings_string_array_add_item")}
			</Button>
		</fieldset>
	);
}

function CodeTextControl({
	config,
	draft,
	language,
	onChange,
	controlId,
}: {
	config: SystemConfig;
	draft: DraftValue;
	language: string;
	onChange: (draft: DraftValue) => void;
	controlId?: string;
}) {
	const { t } = useTranslation();
	const lines = Math.max(6, draft.text.split("\n").length);
	const lineNumbers = useMemo(
		() => Array.from({ length: lines }, (_, index) => index + 1),
		[lines],
	);
	return (
		<div className="overflow-hidden rounded-lg border border-border/70 bg-background dark:border-white/10">
			<div className="flex items-center justify-between border-b border-border/70 bg-muted/35 px-3 py-2 dark:border-white/10">
				<div className="text-xs font-semibold text-muted-foreground">
					{language.toUpperCase()}
				</div>
				{config.is_sensitive ? (
					<span className="text-xs text-muted-foreground">
						{t("settings_sensitive_keep_placeholder")}
					</span>
				) : null}
			</div>
			<div className="grid grid-cols-[3rem_minmax(0,1fr)]">
				<div className="select-none border-r border-border/70 bg-muted/20 py-2 text-right font-mono text-xs leading-5 text-muted-foreground dark:border-white/10">
					{lineNumbers.map((lineNumber) => (
						<div key={`line-${lineNumber}`} className="px-2">
							{lineNumber}
						</div>
					))}
				</div>
				<Textarea
					id={controlId}
					value={draft.text}
					rows={Math.min(18, Math.max(6, lines))}
					placeholder={
						config.is_sensitive ? t("settings_sensitive_keep_placeholder") : ""
					}
					onChange={(event) =>
						onChange({ ...draft, text: event.currentTarget.value })
					}
					className="min-h-40 resize-y rounded-none border-0 bg-transparent font-mono text-xs leading-5 shadow-none focus-visible:ring-0"
				/>
			</div>
		</div>
	);
}

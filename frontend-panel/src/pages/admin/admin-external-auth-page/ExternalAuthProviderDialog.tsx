import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useDialogMotion } from "@/components/ui/dialog-lifecycle";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthKind,
	ExternalAuthProviderKindInfo,
} from "@/types/api";
import { ExternalAuthCreateProgress } from "./ExternalAuthCreateProgress";
import {
	AccessPanel,
	ProviderFormFields,
	ProviderKindPanel,
	ProviderSummaryPanel,
} from "./ExternalAuthProviderFormPanels";
import {
	connectionRequirementsMissing,
	type ExternalAuthCreateStep,
	type ExternalAuthProviderFieldChange,
	type ExternalAuthProviderFormData,
	requiredFieldsMissing,
} from "./shared";

export function ExternalAuthProviderDialog({
	createStep,
	createStepDirection,
	createStepTouched,
	createSteps,
	form,
	mode,
	onCopyCallbackUrl,
	onCreateBack,
	onCreateNext,
	onCreateStepChange,
	onFieldChange,
	onOpenChange,
	onProviderKindChange,
	onSubmit,
	onTestConnection,
	open,
	provider,
	providerKinds,
	submitting,
	testResult,
	testing,
}: {
	createStep: number;
	createStepDirection: "idle" | "forward" | "backward";
	createStepTouched: boolean;
	createSteps: ExternalAuthCreateStep[];
	form: ExternalAuthProviderFormData;
	mode: "create" | "edit";
	onCopyCallbackUrl: (value: string) => void;
	onCreateBack: () => void;
	onCreateNext: () => void;
	onCreateStepChange: (step: number) => void;
	onFieldChange: ExternalAuthProviderFieldChange;
	onOpenChange: (open: boolean) => void;
	onProviderKindChange: (kind: ExternalAuthKind) => void;
	onSubmit: () => void;
	onTestConnection: () => void;
	open: boolean;
	provider: AdminExternalAuthProviderInfo | null;
	providerKinds: ExternalAuthProviderKindInfo[];
	submitting: boolean;
	testResult: string | null;
	testing: boolean;
}) {
	const { t } = useTranslation();
	const isCreate = mode === "create";
	const selectedKind =
		providerKinds.find((kind) => kind.kind === form.providerKind) ??
		providerKinds[0] ??
		null;
	const createLastStep = createSteps.length - 1;
	const submitDisabled =
		submitting || requiredFieldsMissing(form, selectedKind);
	const testDisabled =
		testing || submitting || connectionRequirementsMissing(form, selectedKind);
	const stepPanelClass = cn(
		createStepDirection === "idle"
			? undefined
			: "animate-in fade-in duration-[360ms] motion-reduce:animate-none",
		createStepDirection === "forward"
			? "slide-in-from-right-6"
			: createStepDirection === "backward"
				? "slide-in-from-left-6"
				: undefined,
	);
	// The dialog-lifecycle session above this component owns unmount timing;
	// useDialogMotion reports when the Base UI exit animation has actually
	// finished (via useDialogExitComplete) so the retained content is
	// disposed only after it stops being visible.
	const motionRef = useDialogMotion(open);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				keepMounted
				ref={motionRef.ref}
				className="flex max-h-[min(90dvh,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-[calc(100vw-2rem)] lg:max-w-4xl"
			>
				<DialogHeader className="shrink-0 px-6 pt-5 pr-14 pb-0">
					<DialogTitle>
						{isCreate
							? t("admin.externalAuth.dialog.createTitle")
							: t("admin.externalAuth.dialog.editTitle")}
					</DialogTitle>
					<DialogDescription>
						{t("admin.externalAuth.dialog.description")}
					</DialogDescription>
				</DialogHeader>
				<form
					autoComplete="off"
					className="flex min-h-0 flex-1 flex-col overflow-hidden"
					onSubmit={(event) => {
						event.preventDefault();
						onSubmit();
					}}
				>
					<div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
						{isCreate ? (
							<div className="space-y-5">
								<ExternalAuthCreateProgress
									createStep={createStep}
									createSteps={createSteps}
									onCreateStepChange={onCreateStepChange}
								/>
								<div className="rounded-lg border border-border/70 bg-background/70 p-4">
									<div
										key={`${createStep}-${createStepDirection}`}
										className={stepPanelClass}
									>
										{createStep === 0 ? (
											<ProviderKindPanel
												form={form}
												onCreateStepChange={onCreateStepChange}
												onProviderKindChange={onProviderKindChange}
												providerKinds={providerKinds}
											/>
										) : createStep === 1 ? (
											<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
												<ProviderFormFields
													createStepTouched={createStepTouched}
													form={form}
													onCopyCallbackUrl={onCopyCallbackUrl}
													onFieldChange={onFieldChange}
													onTestConnection={onTestConnection}
													provider={provider}
													selectedKind={selectedKind}
													testDisabled={testDisabled}
													testResult={testResult}
													testing={testing}
												/>
												<ProviderSummaryPanel
													form={form}
													provider={provider}
													providerKinds={providerKinds}
													selectedKind={selectedKind}
												/>
											</div>
										) : (
											<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
												<AccessPanel
													form={form}
													onFieldChange={onFieldChange}
												/>
												<ProviderSummaryPanel
													form={form}
													provider={provider}
													providerKinds={providerKinds}
													selectedKind={selectedKind}
												/>
											</div>
										)}
									</div>
								</div>
							</div>
						) : (
							<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_18rem]">
								<div className="space-y-5">
									<ProviderFormFields
										createStepTouched={createStepTouched}
										form={form}
										onCopyCallbackUrl={onCopyCallbackUrl}
										onFieldChange={onFieldChange}
										onTestConnection={onTestConnection}
										provider={provider}
										selectedKind={selectedKind}
										testDisabled={testDisabled}
										testResult={testResult}
										testing={testing}
									/>
									<AccessPanel form={form} onFieldChange={onFieldChange} />
								</div>
								<ProviderSummaryPanel
									form={form}
									provider={provider}
									providerKinds={providerKinds}
									selectedKind={selectedKind}
								/>
							</div>
						)}
					</div>
					<DialogFooter className="mx-0 w-full shrink-0 flex-row items-center gap-2 rounded-b-xl px-6 py-3">
						<div className="mr-auto">
							{isCreate && createStep > 0 ? (
								<Button
									type="button"
									variant="outline"
									disabled={submitting}
									onClick={onCreateBack}
								>
									{t("common.back")}
								</Button>
							) : (
								<Button
									type="button"
									variant="outline"
									disabled={submitting}
									onClick={() => onOpenChange(false)}
								>
									{t("common.cancel")}
								</Button>
							)}
						</div>
						{isCreate && createStep < createLastStep ? (
							<Button
								type="button"
								disabled={submitting}
								onClick={onCreateNext}
							>
								{t("common.next")}
							</Button>
						) : (
							<Button type="submit" disabled={submitDisabled}>
								{submitting ? (
									<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
								) : (
									<Icon name="FloppyDisk" className="mr-2 size-4" />
								)}
								{isCreate ? t("common.create") : t("common.save")}
							</Button>
						)}
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

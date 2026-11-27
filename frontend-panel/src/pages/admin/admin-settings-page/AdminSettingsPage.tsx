import { useTranslation } from "react-i18next";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { SettingsCategoryContent } from "./SettingsCategoryContent";
import { SettingsCategoryNav } from "./SettingsCategoryNav";
import {
	MailTemplateVariablesDialog,
	TestEmailDialog,
} from "./SettingsDialogs";
import { SettingsSaveBar } from "./SettingsSaveBar";
import {
	type AdminSettingsPageController,
	useAdminSettingsPageController,
} from "./useAdminSettingsPageController";

export default function AdminSettingsPage() {
	const controller = useAdminSettingsPageController();
	return <AdminSettingsPageLayout controller={controller} />;
}

function AdminSettingsPageLayout({
	controller,
}: {
	controller: AdminSettingsPageController;
}) {
	const { t } = useTranslation();
	const {
		active,
		activeTemplateVariableGroup,
		activeTemplateVariableGroupCode,
		categories,
		changedCount,
		configs,
		disabled,
		dispatch,
		drafts,
		empty,
		error,
		expandedTemplateGroups,
		groupedConfigs,
		loading,
		rotatingYggdrasilKey,
		savedAt,
		saving,
		schemaMap,
		sendingTestEmail,
		testEmailDialogOpen,
		testEmailTarget,
	} = controller;

	return (
		<AdminPageShell className="gap-5">
			<AdminPageHeader
				title={t("settings_title")}
				description={t("settings_intro")}
				actions={
					<SettingsActions
						changedCount={changedCount}
						disabled={disabled}
						savedAt={savedAt}
						saving={saving}
						onDiscard={controller.onDiscard}
						onSave={controller.onSave}
					/>
				}
			/>
			<div className="grid gap-5 xl:grid-cols-[16.5rem_minmax(0,1fr)]">
				<SettingsCategoryNav
					active={active}
					categories={categories}
					configs={configs}
				/>
				<SettingsCategoryContent
					drafts={drafts}
					empty={empty}
					expandedTemplateGroups={expandedTemplateGroups}
					groupedConfigs={groupedConfigs}
					loading={loading}
					rotatingYggdrasilKey={rotatingYggdrasilKey}
					schemaMap={schemaMap}
					onChange={controller.onUpdateDraft}
					onOpenTemplateVariables={(value) =>
						dispatch({
							type: "set_active_template_variable_group_code",
							value,
						})
					}
					onOpenTestEmail={() =>
						dispatch({ type: "set_test_email_dialog_open", value: true })
					}
					onRotateYggdrasilSignatureKey={
						controller.onRotateYggdrasilSignatureKey
					}
					onToggleTemplateGroup={(groupKey, open) =>
						dispatch({ type: "toggle_template_group", groupKey, open })
					}
				/>
			</div>
			<SettingsSaveBar
				changedCount={changedCount}
				disabled={disabled}
				error={error}
				hasUnsavedChanges={changedCount > 0}
				saving={saving}
				onDiscard={controller.onDiscard}
				onSave={controller.onSave}
			/>
			<MailTemplateVariablesDialog
				activeGroup={activeTemplateVariableGroup}
				activeGroupCode={activeTemplateVariableGroupCode}
				onOpenChange={(open) =>
					dispatch({
						type: "set_active_template_variable_group_code",
						value: open ? activeTemplateVariableGroupCode : null,
					})
				}
			/>
			<TestEmailDialog
				open={testEmailDialogOpen}
				sending={sendingTestEmail}
				target={testEmailTarget}
				onOpenChange={(value) =>
					dispatch({ type: "set_test_email_dialog_open", value })
				}
				onSend={controller.onSendTestEmail}
				onTargetChange={(value) =>
					dispatch({ type: "set_test_email_target", value })
				}
			/>
		</AdminPageShell>
	);
}

function SettingsActions({
	changedCount,
	disabled,
	onDiscard,
	onSave,
	savedAt,
	saving,
}: {
	changedCount: number;
	disabled: boolean;
	onDiscard: () => void;
	onSave: () => void;
	savedAt: string | null;
	saving: boolean;
}) {
	const { t } = useTranslation();
	return (
		<div className="flex flex-wrap items-center gap-2">
			{savedAt ? (
				<span className="text-xs text-muted-foreground">
					{t("settings_saved_at", {
						time: new Date(savedAt).toLocaleTimeString(),
					})}
				</span>
			) : null}
			<Button
				type="button"
				variant="outline"
				disabled={!changedCount || saving}
				onClick={onDiscard}
			>
				{t("undo_changes")}
			</Button>
			<Button
				type="button"
				disabled={!changedCount || disabled || saving}
				onClick={onSave}
			>
				{saving ? t("settings_saving") : t("save_changes")}
			</Button>
		</div>
	);
}

import { useTranslation } from "react-i18next";
import { LoginDevicesSection } from "@/components/settings/LoginDevicesSection";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { EmailChangeSection } from "@/pages/account/account-settings-page/EmailChangeSection";
import {
	DetailRow,
	SectionBlock,
} from "@/pages/account/account-settings-page/settingsSection";
import type { AuthUserInfo } from "@/types/api";

export function AccountSecuritySection({ user }: { user: AuthUserInfo }) {
	const { t } = useTranslation();
	const avatarSource = user.profile?.avatar.source ?? "none";
	const statusLabel =
		t(`personalSettings.status_${user.status}`, {
			defaultValue: user.status,
		}) || user.status;

	const accountRows = [
		{
			label: t("personalSettings.username"),
			value: user.username || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.email"),
			value: user.email || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.role"),
			value: user.role || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.accountStatus"),
			value: statusLabel,
		},
		{
			label: t("personalSettings.avatarSource"),
			value: t(`personalSettings.avatarSource_${avatarSource}`),
		},
	] as const;

	return (
		<SectionBlock
			id="security"
			icon="Shield"
			title={t("personalSettings.sections.security")}
			description={t("personalSettings.sectionDescriptions.security")}
		>
			<div className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
				<div className="grid gap-2 @min-[44rem]/settings:grid-cols-2 @min-[90rem]/settings:grid-cols-3">
					{accountRows.map((row) => (
						<DetailRow
							key={row.label}
							label={row.label}
							value={
								row.label === t("personalSettings.accountStatus") ? (
									<Badge
										variant="outline"
										className="h-6 w-fit rounded-md px-2"
									>
										{row.value}
									</Badge>
								) : (
									<span className="block min-w-0 truncate">{row.value}</span>
								)
							}
						/>
					))}
				</div>
				<Separator className="my-4" />
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
					<div className="text-sm leading-6 text-muted-foreground">
						{t("personalSettings.securityDescription")}
					</div>
				</div>
				<div className="mt-4">
					<EmailChangeSection user={user} />
				</div>
				<div className="mt-4">
					<LoginDevicesSection />
				</div>
			</div>
		</SectionBlock>
	);
}

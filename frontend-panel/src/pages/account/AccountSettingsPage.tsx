import { useTranslation } from "react-i18next";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { SecurityPasskeysSection } from "@/components/settings/SecurityPasskeysSection";
import { Skeleton } from "@/components/ui/skeleton";
import { usePageTitle } from "@/hooks/usePageTitle";
import { AccountSecuritySection } from "@/pages/account/account-settings-page/AccountSecuritySection";
import { ExternalAuthLinksSection } from "@/pages/account/account-settings-page/ExternalAuthLinksSection";
import { ProfileSection } from "@/pages/account/account-settings-page/ProfileSection";
import {
	SectionBlock,
	SectionNav,
	scrollToSettingsSection,
} from "@/pages/account/account-settings-page/settingsSection";
import { useAccountSettingsPageController } from "@/pages/account/account-settings-page/useAccountSettingsPageController";

/**
 * Personal settings page shell. Section state machines live in colocated
 * files under `account-settings-page/`; the passive sections
 * (LoginDevicesSection / SecurityPasskeysSection) stay untouched.
 */
export default function AccountSettingsPage() {
	const { t } = useTranslation();
	const { activeSection, sections, user } = useAccountSettingsPageController();

	usePageTitle(t("personalSettings.title"));

	if (!user) {
		return (
			<div className="mx-auto w-full max-w-[96rem] px-4 py-5 sm:px-6 lg:px-7">
				<div
					className="rounded-lg border border-border/70 bg-card p-5 text-card-foreground shadow-sm dark:border-white/10 dark:bg-card/90 dark:shadow-none"
					role="status"
				>
					<span className="sr-only">{t("common.loading")}</span>
					<div className="space-y-4">
						<div className="flex items-center gap-3">
							<Skeleton className="size-12 rounded-full" />
							<div className="flex-1 space-y-2">
								<Skeleton className="h-4 w-1/3" />
								<Skeleton className="h-3 w-1/2" />
							</div>
						</div>
						<Skeleton className="h-32 w-full rounded-lg" />
						<Skeleton className="h-24 w-full rounded-lg" />
					</div>
				</div>
			</div>
		);
	}

	const profileName = user.profile?.display_name?.trim() || user.username;
	const userProfileKey = `${user.id}:${user.profile?.display_name ?? ""}`;

	return (
		<div className="@container/settings">
			<div className="mx-auto grid w-full max-w-[104rem] gap-6 px-4 py-5 sm:px-6 lg:px-7 @min-[56rem]/settings:grid-cols-[minmax(0,1fr)_15rem]">
				<div className="min-w-0">
					<div className="mb-5 border-b border-border/70 pb-5 dark:border-white/10">
						<div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
							<div className="min-w-0">
								<h1 className="text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
									{t("personalSettings.title")}
								</h1>
								<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
									{t("personalSettings.description")}
								</p>
							</div>
							<div className="flex items-center gap-3 rounded-lg border border-border/70 bg-card px-3 py-2 dark:border-white/10 dark:bg-card/90">
								<UserAvatarImage
									name={profileName}
									avatar={user.profile?.avatar}
									size="md"
								/>
								<div className="min-w-0">
									<div className="truncate text-sm font-semibold">
										{profileName}
									</div>
									<div className="truncate text-xs text-muted-foreground">
										{user.email}
									</div>
								</div>
							</div>
						</div>
					</div>

					<div className="min-w-0">
						<ProfileSection key={userProfileKey} user={user} />

						<AccountSecuritySection user={user} />

						<SectionBlock
							id="passkeys"
							icon="Key"
							title={t("personalSettings.sections.passkeys")}
							description={t("personalSettings.sectionDescriptions.passkeys")}
						>
							<SecurityPasskeysSection />
						</SectionBlock>

						<SectionBlock
							id="external-auth"
							icon="LinkSimple"
							title={t("personalSettings.sections.externalAuth")}
							description={t(
								"personalSettings.sectionDescriptions.externalAuth",
							)}
						>
							<ExternalAuthLinksSection />
						</SectionBlock>
					</div>
				</div>

				<aside className="hidden @min-[56rem]/settings:block">
					<div className="sticky top-20 rounded-lg border border-border/70 bg-card p-2 shadow-sm dark:border-white/10 dark:bg-card/90 dark:shadow-none">
						<div className="px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
							{t("personalSettings.sectionNav")}
						</div>
						<SectionNav
							sections={sections}
							activeSection={activeSection}
							onSelect={scrollToSettingsSection}
						/>
					</div>
				</aside>
			</div>
		</div>
	);
}

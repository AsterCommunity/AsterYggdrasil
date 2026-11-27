import { useMinecraftProfilesPageController } from "@/components/account/profiles-page/useMinecraftProfilesPageController";
import {
	ProfileDeleteDialog,
	ProfileRenameDialog,
	ProfileTextureDialogs,
} from "@/pages/account/minecraft-profiles-page/profileDialogs";
import {
	ProfileListSection,
	ProfilesPageHeader,
} from "@/pages/account/minecraft-profiles-page/profileListSection";
import { ProfilePreviewPanel } from "@/pages/account/minecraft-profiles-page/profilePreviewPanel";

/**
 * Minecraft profiles page shell. The state machine lives in
 * `components/account/profiles-page/useMinecraftProfilesPageController.ts`;
 * this file only assembles the colocated section components.
 */
export default function MinecraftProfilesPage() {
	const controller = useMinecraftProfilesPageController();

	return (
		<div className="mx-auto w-full max-w-[96rem] px-4 py-5 sm:px-6 lg:px-7">
			<ProfilesPageHeader />
			<div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
				<ProfileListSection controller={controller} />
				<ProfilePreviewPanel controller={controller} />
			</div>
			<ProfileTextureDialogs controller={controller} />
			<ProfileRenameDialog controller={controller} />
			<ProfileDeleteDialog controller={controller} />
		</div>
	);
}

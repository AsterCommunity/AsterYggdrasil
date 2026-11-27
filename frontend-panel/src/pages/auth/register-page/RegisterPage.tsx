import { authEntryMainClassName } from "@/components/auth/AuthFormPrimitives";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import { LoginFormCard } from "@/components/auth/LoginFormCard";
import { LoginHero } from "@/components/auth/LoginHero";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { useRegisterPageController } from "@/pages/auth/register-page/useRegisterPageController";

/**
 * Registration page shell. The registration state machine lives in
 * `useRegisterPageController.ts`; login is a separate page
 * (`@/pages/LoginPage`).
 */
export default function RegisterPage() {
	const {
		branding,
		brandTitle,
		description,
		formCardProps,
		headline,
		tagline,
	} = useRegisterPageController();

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={tagline}
			variant="auth"
		>
			<main className={authEntryMainClassName}>
				<LoginHero
					isRegister={true}
					headline={headline}
					description={description}
				/>
				<LoginFormCard {...formCardProps} />
			</main>

			<LoginEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}

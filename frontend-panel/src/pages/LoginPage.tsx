import { authEntryMainClassName } from "@/components/auth/AuthFormPrimitives";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import { LoginFormCard } from "@/components/auth/LoginFormCard";
import { LoginHero } from "@/components/auth/LoginHero";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { useLoginPageController } from "@/pages/auth/login-page/useLoginPageController";

/**
 * Login page shell. The login + external-auth recovery state machine lives
 * in `auth/login-page/useLoginPageController.ts`; registration is a
 * separate page (`auth/register-page/RegisterPage.tsx`).
 */
export default function LoginPage() {
	const {
		branding,
		brandTitle,
		description,
		formCardProps,
		headline,
		tagline,
	} = useLoginPageController();

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={tagline}
			variant="auth"
		>
			<main className={authEntryMainClassName}>
				<LoginHero
					isRegister={false}
					headline={headline}
					description={description}
				/>
				<LoginFormCard {...formCardProps} />
			</main>

			<LoginEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}

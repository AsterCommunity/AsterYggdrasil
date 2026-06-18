import type { FormEvent } from "react";
import { useReducer } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import { passwordChangeSchema } from "@/lib/validation";
import { Loading } from "@/router/Loading";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type PasswordChangeValues = {
	confirmPassword: string;
	currentPassword: string;
	newPassword: string;
};

type PasswordChangeTouched = Record<keyof PasswordChangeValues, boolean>;
type PasswordChangeErrors = Partial<Record<keyof PasswordChangeValues, string>>;

const initialValues: PasswordChangeValues = {
	confirmPassword: "",
	currentPassword: "",
	newPassword: "",
};

const initialTouched: PasswordChangeTouched = {
	confirmPassword: false,
	currentPassword: false,
	newPassword: false,
};

const allTouched: PasswordChangeTouched = {
	confirmPassword: true,
	currentPassword: true,
	newPassword: true,
};

type PasswordChangeState = {
	errors: PasswordChangeErrors;
	showPasswords: boolean;
	signingOut: boolean;
	submitting: boolean;
	touched: PasswordChangeTouched;
	values: PasswordChangeValues;
};

type PasswordChangeAction =
	| { type: "field"; field: keyof PasswordChangeValues; value: string }
	| { type: "touchAll" }
	| { type: "togglePasswords" }
	| { type: "submitting"; value: boolean }
	| { type: "signingOut"; value: boolean };

const initialState: PasswordChangeState = {
	errors: {},
	showPasswords: false,
	signingOut: false,
	submitting: false,
	touched: initialTouched,
	values: initialValues,
};

function passwordChangeErrors(
	values: PasswordChangeValues,
	touched: PasswordChangeTouched,
) {
	const result = passwordChangeSchema.safeParse(values);
	if (result.success) return {};

	const next: PasswordChangeErrors = {};
	for (const issue of result.error.issues) {
		const field = issue.path[0];
		if (
			(field === "confirmPassword" ||
				field === "currentPassword" ||
				field === "newPassword") &&
			touched[field]
		) {
			next[field] ??= issue.message;
		}
	}
	return next;
}

function reducer(
	state: PasswordChangeState,
	action: PasswordChangeAction,
): PasswordChangeState {
	switch (action.type) {
		case "field": {
			const nextValues = { ...state.values, [action.field]: action.value };
			const nextTouched = { ...state.touched, [action.field]: true };
			return {
				...state,
				values: nextValues,
				touched: nextTouched,
				errors: passwordChangeErrors(nextValues, nextTouched),
			};
		}
		case "touchAll":
			return {
				...state,
				touched: allTouched,
				errors: passwordChangeErrors(state.values, allTouched),
			};
		case "togglePasswords":
			return { ...state, showPasswords: !state.showPasswords };
		case "submitting":
			return { ...state, submitting: action.value };
		case "signingOut":
			return { ...state, signingOut: action.value };
	}
}

export default function ForcePasswordChangePage() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const branding = useFrontendConfigStore((state) => state.branding);
	const checking = useAuthStore((state) => state.checking);
	const changePassword = useAuthStore((state) => state.changePassword);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const logout = useAuthStore((state) => state.logout);
	const mustChangePassword = useAuthStore(
		(state) => state.user?.must_change_password ?? false,
	);
	const [state, dispatch] = useReducer(reducer, initialState);
	const { errors, showPasswords, signingOut, submitting, values } = state;
	const brandTitle = branding.title || t("brand.name");
	const canSubmit = passwordChangeSchema.safeParse(values).success;

	usePageTitle(t("login.forcePasswordChangeTitle"));

	if (checking) return <Loading surface="public" />;
	if (!isAuthenticated) return <Navigate to={publicPaths.login} replace />;
	if (!mustChangePassword) {
		return <Navigate to={accountPaths.home} replace />;
	}

	function updateField(field: keyof PasswordChangeValues, value: string) {
		dispatch({ type: "field", field, value });
	}

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const validation = passwordChangeSchema.safeParse(values);
		dispatch({ type: "touchAll" });
		if (!validation.success) {
			toast.error(t("login.validationFailed"));
			return;
		}

		dispatch({ type: "submitting", value: true });
		try {
			await changePassword(
				validation.data.currentPassword,
				validation.data.newPassword,
			);
			toast.success(t("login.forcePasswordChangeSuccess"));
			navigate(accountPaths.home, { replace: true });
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function signOut() {
		dispatch({ type: "signingOut", value: true });
		try {
			await logout();
			navigate(publicPaths.login, { replace: true });
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "signingOut", value: false });
		}
	}

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={t("brand.tagline")}
			variant="auth"
			hideLanguageOnMobile
		>
			<main className="mx-auto flex w-full max-w-[92rem] flex-1 items-center px-4 pb-10 sm:px-8 lg:px-12">
				<section className="w-full max-w-md rounded-lg border border-white/70 bg-white/88 p-6 shadow-lg backdrop-blur-md dark:border-white/10 dark:bg-[#07110d]/84">
					<div className="mb-6 space-y-2">
						<p className="font-medium text-emerald-700 text-sm dark:text-emerald-300">
							{t("login.forcePasswordChangeEyebrow")}
						</p>
						<h1 className="font-semibold text-2xl tracking-normal">
							{t("login.forcePasswordChangeTitle")}
						</h1>
						<p className="text-muted-foreground text-sm leading-6">
							{t("login.forcePasswordChangeDescription")}
						</p>
					</div>
					<form className="space-y-4" onSubmit={submit}>
						<PasswordField
							id="force-current-password"
							label={t("login.currentPassword")}
							value={values.currentPassword}
							error={errors.currentPassword && t(errors.currentPassword)}
							autoComplete="current-password"
							showPassword={showPasswords}
							onChange={(value) => updateField("currentPassword", value)}
						/>
						<PasswordField
							id="force-new-password"
							label={t("login.newPassword")}
							value={values.newPassword}
							error={errors.newPassword && t(errors.newPassword)}
							description={
								errors.newPassword
									? undefined
									: t("admin.users.passwordCreateHint")
							}
							autoComplete="new-password"
							showPassword={showPasswords}
							onChange={(value) => updateField("newPassword", value)}
						/>
						<PasswordField
							id="force-confirm-password"
							label={t("login.confirmPassword")}
							value={values.confirmPassword}
							error={errors.confirmPassword && t(errors.confirmPassword)}
							autoComplete="new-password"
							showPassword={showPasswords}
							onChange={(value) => updateField("confirmPassword", value)}
						/>
						<div className="flex flex-col-reverse gap-3 pt-1 sm:flex-row sm:items-center sm:justify-between">
							<Button
								type="button"
								variant="ghost"
								onClick={() => dispatch({ type: "togglePasswords" })}
							>
								<Icon
									name={showPasswords ? "EyeSlash" : "Eye"}
									className="mr-2 size-4"
								/>
								{showPasswords
									? t("login.hidePassword")
									: t("login.showPassword")}
							</Button>
							<Button
								type="submit"
								className="min-w-36"
								disabled={submitting || !canSubmit}
							>
								<Icon
									name={submitting ? "Spinner" : "Key"}
									className={
										submitting ? "mr-2 size-4 animate-spin" : "mr-2 size-4"
									}
								/>
								{t("login.forcePasswordChangeSubmit")}
							</Button>
						</div>
					</form>
					<div className="mt-5 border-border/70 border-t pt-4 dark:border-white/10">
						<Button
							type="button"
							variant="outline"
							className="w-full"
							disabled={signingOut}
							onClick={() => void signOut()}
						>
							<Icon
								name={signingOut ? "Spinner" : "SignOut"}
								className={
									signingOut ? "mr-2 size-4 animate-spin" : "mr-2 size-4"
								}
							/>
							{t("nav.logout")}
						</Button>
					</div>
				</section>
			</main>
		</PublicEntryShell>
	);
}

function PasswordField({
	autoComplete,
	description,
	error,
	id,
	label,
	onChange,
	showPassword,
	value,
}: {
	autoComplete: string;
	description?: string;
	error?: string;
	id: string;
	label: string;
	onChange: (value: string) => void;
	showPassword: boolean;
	value: string;
}) {
	return (
		<div className="space-y-2">
			<Label htmlFor={id}>{label}</Label>
			<Input
				id={id}
				type={showPassword ? "text" : "password"}
				value={value}
				autoComplete={autoComplete}
				aria-invalid={error ? true : undefined}
				onChange={(event) => onChange(event.target.value)}
			/>
			{error ? (
				<p className="text-destructive text-sm">{error}</p>
			) : description ? (
				<p className="text-muted-foreground text-xs leading-5">{description}</p>
			) : null}
		</div>
	);
}

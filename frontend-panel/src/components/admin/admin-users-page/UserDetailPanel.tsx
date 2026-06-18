import { useEffect, useReducer, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { handleApiError } from "@/hooks/useApiError";
import { passwordSchema } from "@/lib/validation";
import { adminMinecraftProfileService } from "@/services/adminService";
import type {
	AdminUserInfo,
	MinecraftTextureMetadata,
	OperatorScope,
	UpdateAdminUserRequest,
	UserRole,
	UserStatus,
} from "@/types/api";
import { UserDetailAccountSection } from "./UserDetailAccountSection";
import { UserDetailFooterActions } from "./UserDetailFooterActions";
import {
	UserDetailMinecraftSection,
	type UserMinecraftProfileItem,
} from "./UserDetailMinecraftSection";
import { UserDetailSecuritySection } from "./UserDetailSecuritySection";
import { UserDetailSidebar } from "./UserDetailSidebar";

const MINECRAFT_PROFILE_PAGE_SIZE_OPTIONS = [5, 10, 20] as const;
const DEFAULT_MINECRAFT_PROFILE_PAGE_SIZE =
	MINECRAFT_PROFILE_PAGE_SIZE_OPTIONS[0];

type BusyField =
	| "revokingSessions"
	| "savingForcePasswordChange"
	| "savingPassword"
	| "savingProfile";

type UserDetailDraftState = {
	confirmPassword: string;
	email: string;
	password: string;
	passwordError?: string;
	confirmPasswordError?: string;
	revokingSessions: boolean;
	savingForcePasswordChange: boolean;
	operatorScopes: OperatorScope[];
	role: UserRole;
	savingPassword: boolean;
	savingProfile: boolean;
	status: UserStatus;
	username: string;
};

type UserDetailDraftAction =
	| { type: "clear_password_error"; field: "confirmPassword" | "password" }
	| { type: "password_reset_success" }
	| { type: "set_busy"; field: BusyField; value: boolean }
	| { type: "set_confirm_password"; value: string }
	| { type: "set_email"; value: string }
	| { type: "set_password"; value: string }
	| {
			type: "set_password_errors";
			errors: { confirmPassword?: string; password?: string };
	  }
	| { type: "set_role"; value: UserRole }
	| { type: "set_operator_scopes"; value: OperatorScope[] }
	| { type: "set_status"; value: UserStatus }
	| { type: "set_username"; value: string };

type UserDetailPanelProps = {
	onBack?: () => void;
	onRevokeSessions: (id: number) => Promise<number>;
	onUpdate: (id: number, data: UpdateAdminUserRequest) => Promise<void>;
	user: AdminUserInfo;
};

function createUserDraftState(user: AdminUserInfo): UserDetailDraftState {
	return {
		confirmPassword: "",
		email: user.email,
		password: "",
		revokingSessions: false,
		savingForcePasswordChange: false,
		operatorScopes: user.operator_scopes ?? [],
		role: user.role,
		savingPassword: false,
		savingProfile: false,
		status: user.status,
		username: user.username,
	};
}

function userDetailDraftReducer(
	state: UserDetailDraftState,
	action: UserDetailDraftAction,
): UserDetailDraftState {
	switch (action.type) {
		case "clear_password_error":
			return {
				...state,
				[`${action.field}Error`]: undefined,
			};
		case "password_reset_success":
			return {
				...state,
				confirmPassword: "",
				confirmPasswordError: undefined,
				password: "",
				passwordError: undefined,
			};
		case "set_busy":
			return {
				...state,
				[action.field]: action.value,
			};
		case "set_confirm_password":
			return {
				...state,
				confirmPassword: action.value,
			};
		case "set_email":
			return {
				...state,
				email: action.value,
			};
		case "set_password":
			return {
				...state,
				password: action.value,
			};
		case "set_password_errors":
			return {
				...state,
				confirmPasswordError: action.errors.confirmPassword,
				passwordError: action.errors.password,
			};
		case "set_role":
			return {
				...state,
				role: action.value,
			};
		case "set_operator_scopes":
			return {
				...state,
				operatorScopes: action.value,
			};
		case "set_status":
			return {
				...state,
				status: action.value,
			};
		case "set_username":
			return {
				...state,
				username: action.value,
			};
	}
}

function userDetailDraftKey(user: AdminUserInfo) {
	return [
		user.id,
		user.username,
		user.email,
		(user.operator_scopes ?? []).join(","),
		user.role,
		user.status,
		user.active_session_count,
		user.profile_count,
		user.session_version,
		user.must_change_password,
	].join(":");
}

async function attachMinecraftProfileSkinUrls(
	profiles: Array<{ id: string; name: string }>,
): Promise<UserMinecraftProfileItem[]> {
	return Promise.all(
		profiles.map(async (profile) => {
			try {
				const textures = await adminMinecraftProfileService.listTextures(
					profile.id,
				);
				return {
					...profile,
					skinUrl: findBoundSkinTexture(textures)?.url ?? null,
				};
			} catch (error) {
				console.warn("Failed to load Minecraft profile skin avatar", error);
				return { ...profile, skinUrl: null };
			}
		}),
	);
}

function findBoundSkinTexture(textures: MinecraftTextureMetadata[]) {
	return textures.find(
		(texture) => texture.texture_type === "skin" && texture.source === "bound",
	);
}

export function UserDetailPanel({
	onBack,
	onRevokeSessions,
	onUpdate,
	user,
}: UserDetailPanelProps) {
	const { t } = useTranslation();
	const [state, dispatch] = useReducer(
		userDetailDraftReducer,
		user,
		createUserDraftState,
	);
	const [minecraftProfiles, setMinecraftProfiles] = useState<
		UserMinecraftProfileItem[]
	>([]);
	const [minecraftProfileTotal, setMinecraftProfileTotal] = useState(0);
	const [minecraftProfileOffset, setMinecraftProfileOffset] = useState(0);
	const [minecraftProfilePageSize, setMinecraftProfilePageSize] = useState<
		(typeof MINECRAFT_PROFILE_PAGE_SIZE_OPTIONS)[number]
	>(DEFAULT_MINECRAFT_PROFILE_PAGE_SIZE);
	const previousMinecraftProfileUserIdRef = useRef(user.id);
	const [profilesLoading, setProfilesLoading] = useState(true);
	const {
		confirmPassword,
		confirmPasswordError,
		email,
		operatorScopes,
		password,
		passwordError,
		revokingSessions,
		savingForcePasswordChange,
		role,
		savingPassword,
		savingProfile,
		status,
		username,
	} = state;
	const roleStatusLocked = user.id === 1;
	const hasProfileChanges =
		username.trim() !== user.username ||
		email.trim() !== user.email ||
		(!roleStatusLocked && role !== user.role) ||
		(!roleStatusLocked &&
			role === "operator" &&
			operatorScopes.join(",") !== (user.operator_scopes ?? []).join(",")) ||
		(!roleStatusLocked && status !== user.status);
	const profileInvalid = !username.trim() || !email.trim();
	const busy =
		savingProfile ||
		savingPassword ||
		revokingSessions ||
		savingForcePasswordChange;

	useEffect(() => {
		if (previousMinecraftProfileUserIdRef.current === user.id) return;
		previousMinecraftProfileUserIdRef.current = user.id;
		setMinecraftProfileOffset(0);
	}, [user.id]);

	useEffect(() => {
		let cancelled = false;
		async function loadProfiles() {
			try {
				setProfilesLoading(true);
				const page = await adminMinecraftProfileService.listByUserPage(
					user.id,
					{
						limit: minecraftProfilePageSize,
						offset: minecraftProfileOffset,
					},
				);
				if (cancelled) return;
				if (
					page.items.length === 0 &&
					page.total > 0 &&
					minecraftProfileOffset > 0
				) {
					setMinecraftProfileOffset(
						Math.max(
							0,
							Math.floor((page.total - 1) / minecraftProfilePageSize) *
								minecraftProfilePageSize,
						),
					);
					return;
				}
				const nextProfiles = await attachMinecraftProfileSkinUrls(
					page.items.map((item) => ({ id: item.id, name: item.name })),
				);
				if (cancelled) return;
				setMinecraftProfiles(nextProfiles);
				setMinecraftProfileTotal(page.total);
			} catch (error) {
				if (!cancelled) handleApiError(error);
			} finally {
				if (!cancelled) setProfilesLoading(false);
			}
		}
		void loadProfiles();
		return () => {
			cancelled = true;
		};
	}, [minecraftProfileOffset, minecraftProfilePageSize, user.id]);

	const runPanelAction = async (
		field: BusyField,
		action: () => Promise<void>,
		successMessage?: string,
	) => {
		try {
			dispatch({ type: "set_busy", field, value: true });
			await action();
			if (successMessage) toast.success(successMessage);
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_busy", field, value: false });
		}
	};

	const handleProfileSave = async () => {
		if (!hasProfileChanges || profileInvalid) return;

		const data: UpdateAdminUserRequest = {};
		const nextUsername = username.trim();
		const nextEmail = email.trim();

		if (nextUsername !== user.username) data.username = nextUsername;
		if (nextEmail !== user.email) data.email = nextEmail;
		if (!roleStatusLocked && role !== user.role) data.role = role;
		if (
			!roleStatusLocked &&
			role === "operator" &&
			(operatorScopes.join(",") !== (user.operator_scopes ?? []).join(",") ||
				role !== user.role)
		) {
			data.operator_scopes = operatorScopes;
		}
		if (!roleStatusLocked && status !== user.status) data.status = status;

		await runPanelAction(
			"savingProfile",
			async () => onUpdate(user.id, data),
			t("admin.users.updated"),
		);
	};

	const handlePasswordReset = async () => {
		const nextPassword = password.trim();
		const errors: { confirmPassword?: string; password?: string } = {};

		const passwordResult = passwordSchema.safeParse(nextPassword);
		if (!passwordResult.success) {
			errors.password = t(passwordResult.error.issues[0]?.message ?? "");
		}
		if (confirmPassword !== password) {
			errors.confirmPassword = t("admin.users.passwordConfirmMismatch");
		}

		dispatch({ type: "set_password_errors", errors });
		if (Object.keys(errors).length > 0) return;

		await runPanelAction(
			"savingPassword",
			async () => {
				await onUpdate(user.id, { password: nextPassword });
				dispatch({ type: "password_reset_success" });
			},
			t("admin.users.passwordResetSuccess"),
		);
	};

	const handleSessionRevoke = async () => {
		await runPanelAction("revokingSessions", async () => {
			const removed = await onRevokeSessions(user.id);
			toast.success(t("admin.users.sessionsRevoked", { count: removed }));
		});
	};

	const handleForcePasswordChangeToggle = async (value: boolean) => {
		if (value === user.must_change_password) return;
		await runPanelAction(
			"savingForcePasswordChange",
			async () => onUpdate(user.id, { must_change_password: value }),
			t("admin.users.updated"),
		);
	};

	return (
		<div
			key={userDetailDraftKey(user)}
			className="overflow-hidden rounded-lg border border-border/70 bg-card text-card-foreground shadow-xs dark:border-white/10 dark:bg-card/90 dark:shadow-none"
		>
			<div className="grid min-h-[calc(100dvh-13rem)] lg:grid-cols-[20rem_minmax(0,1fr)]">
				<UserDetailSidebar user={user} />
				<div className="flex min-w-0 flex-col">
					<div className="border-b border-border/70 px-5 py-4 dark:border-white/10">
						<div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
							<div>
								<h2 className="text-lg font-semibold">
									{t("admin.users.detailTitle")}
								</h2>
								<p className="mt-1 text-sm leading-6 text-muted-foreground">
									{t("admin.users.detailDescription")}
								</p>
							</div>
							{hasProfileChanges ? (
								<Badge variant="outline" className="w-fit rounded-md">
									{t("admin.users.unsavedChanges")}
								</Badge>
							) : null}
						</div>
					</div>
					<div className="flex-1 px-5 py-5">
						<div className="grid gap-5">
							<UserDetailAccountSection
								email={email}
								operatorScopes={operatorScopes}
								role={role}
								roleStatusLocked={roleStatusLocked}
								savingProfile={savingProfile}
								status={status}
								username={username}
								onEmailChange={(value) =>
									dispatch({ type: "set_email", value })
								}
								onOperatorScopesChange={(value) =>
									dispatch({ type: "set_operator_scopes", value })
								}
								onRoleChange={(value) => dispatch({ type: "set_role", value })}
								onStatusChange={(value) =>
									dispatch({ type: "set_status", value })
								}
								onUsernameChange={(value) =>
									dispatch({ type: "set_username", value })
								}
							/>
							<UserDetailSecuritySection
								activeSessionCount={user.active_session_count}
								confirmPassword={confirmPassword}
								confirmPasswordError={confirmPasswordError}
								mustChangePassword={user.must_change_password}
								password={password}
								passwordError={passwordError}
								revokingSessions={revokingSessions}
								savingForcePasswordChange={savingForcePasswordChange}
								savingPassword={savingPassword}
								onConfirmPasswordChange={(value) => {
									dispatch({ type: "set_confirm_password", value });
									dispatch({
										type: "clear_password_error",
										field: "confirmPassword",
									});
								}}
								onPasswordChange={(value) => {
									dispatch({ type: "set_password", value });
									dispatch({ type: "clear_password_error", field: "password" });
								}}
								onPasswordReset={() => void handlePasswordReset()}
								onForcePasswordChangeToggle={(value) =>
									void handleForcePasswordChangeToggle(value)
								}
								onSessionRevoke={() => void handleSessionRevoke()}
							/>
							<UserDetailMinecraftSection
								currentPage={
									Math.floor(
										minecraftProfileOffset / minecraftProfilePageSize,
									) + 1
								}
								loading={profilesLoading}
								offset={minecraftProfileOffset}
								pageSize={minecraftProfilePageSize}
								pageSizeOptions={MINECRAFT_PROFILE_PAGE_SIZE_OPTIONS.map(
									(size) => ({
										label: t("admin.pagination.pageSizeOption", {
											count: size,
										}),
										value: String(size),
									}),
								)}
								profiles={minecraftProfiles}
								total={minecraftProfileTotal}
								totalPages={Math.max(
									1,
									Math.ceil(minecraftProfileTotal / minecraftProfilePageSize),
								)}
								userId={user.id}
								onNext={() =>
									setMinecraftProfileOffset(
										(current) => current + minecraftProfilePageSize,
									)
								}
								onPageSizeChange={(value) => {
									const next = MINECRAFT_PROFILE_PAGE_SIZE_OPTIONS.find(
										(size) => String(size) === value,
									);
									if (!next) return;
									setMinecraftProfilePageSize(next);
									setMinecraftProfileOffset(0);
								}}
								onPrevious={() =>
									setMinecraftProfileOffset((current) =>
										Math.max(0, current - minecraftProfilePageSize),
									)
								}
							/>
						</div>
					</div>
					<UserDetailFooterActions
						busy={busy}
						hasProfileChanges={hasProfileChanges}
						profileInvalid={profileInvalid}
						savingProfile={savingProfile}
						onBack={onBack}
						onSave={() => void handleProfileSave()}
					/>
				</div>
			</div>
		</div>
	);
}

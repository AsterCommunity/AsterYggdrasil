export const publicPaths = {
	home: "/",
	login: "/login",
	register: "/register",
	resetPassword: "/reset-password",
	textureLibrary: "/textures",
	textureLibraryDetail: "/textures/:textureId",
	invite: "/invite/:token",
	init: "/init",
	tos: "/tos",
	privacy: "/privacy",
} as const;

export function publicTexturePath(textureId: number | string) {
	return `/textures/${encodeURIComponent(String(textureId))}`;
}

export const accountPaths = {
	home: "/account",
	forcePasswordChange: "/force-password-change",
	profiles: "/account/profiles",
	wardrobe: "/account/wardrobe",
	audit: "/account/audit",
	settings: "/account/settings",
	settingsSecurityCompat: "/settings/security",
} as const;

export const adminPaths = {
	home: "/admin",
	overview: "/admin",
	users: "/admin/users",
	userInvitations: "/admin/users/invitations",
	externalAuth: "/admin/external-auth",
	textureLibrary: "/admin/texture-library",
	audit: "/admin/audit",
	tasks: "/admin/tasks",
	settings: "/admin/settings",
	about: "/admin/about",
	minecraftProfile: "/admin/minecraft-profiles/:uuid",
	userDetail: "/admin/users/:id",
} as const;

export function adminMinecraftProfilePath(uuid: string) {
	return `/admin/minecraft-profiles/${encodeURIComponent(uuid)}`;
}

export function adminUserPath(id: number | string) {
	return `/admin/users/${encodeURIComponent(String(id))}`;
}

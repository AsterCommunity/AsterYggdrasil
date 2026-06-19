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
	yggdrasilForwarding: "/admin/yggdrasil-forwarding",
	textureLibrary: "/admin/texture-library",
	textureLibraryDetail: "/admin/texture-library/:textureId",
	textureLibraryReviews: "/admin/texture-library/reviews",
	textureLibraryReports: "/admin/texture-library/reports",
	textureLibraryTags: "/admin/texture-library/tags",
	audit: "/admin/audit",
	tasks: "/admin/tasks",
	settings: "/admin/settings",
	settingsCategory: "/admin/settings/:category",
	about: "/admin/about",
	minecraftProfile: "/admin/minecraft-profiles/:uuid",
	userDetail: "/admin/users/:id",
} as const;

export function adminMinecraftProfilePath(uuid: string) {
	return `/admin/minecraft-profiles/${encodeURIComponent(uuid)}`;
}

export function adminTextureLibraryPath(textureId: number | string) {
	return `/admin/texture-library/${encodeURIComponent(String(textureId))}`;
}

export function adminUserPath(id: number | string) {
	return `/admin/users/${encodeURIComponent(String(id))}`;
}

export function adminSettingsCategoryPath(category: string) {
	return `/admin/settings/${encodeURIComponent(category)}`;
}

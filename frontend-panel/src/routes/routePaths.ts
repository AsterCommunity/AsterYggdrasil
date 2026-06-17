export const publicPaths = {
	home: "/",
	login: "/login",
	register: "/register",
	init: "/init",
	tos: "/tos",
	privacy: "/privacy",
} as const;

export const accountPaths = {
	home: "/account",
	profiles: "/account/profiles",
	wardrobe: "/account/wardrobe",
	audit: "/account/audit",
	settings: "/account/settings",
} as const;

export const adminPaths = {
	home: "/admin",
	users: "/admin/users",
	externalAuth: "/admin/external-auth",
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

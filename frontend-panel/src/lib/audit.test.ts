import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createInstance } from "i18next";
import { describe, expect, it } from "vitest";
import { resources } from "@/i18n/resources";
import {
	formatAuditAction,
	formatAuditDetail,
	formatAuditEntityType,
	formatAuditSummary,
	formatAuditTarget,
	getAuditActionBadgeClass,
} from "@/lib/audit";
import type { AuditLogEntry } from "@/types/api";

type AuditEntrySample = Pick<
	AuditLogEntry,
	"action" | "entity_id" | "entity_name" | "entity_type" | "presentation"
>;

async function tFor(language: keyof typeof resources) {
	const i18n = createInstance();
	await i18n.init({
		defaultNS: "frontend",
		fallbackLng: "en-US",
		interpolation: {
			escapeValue: false,
		},
		lng: language,
		resources,
	});
	return i18n.t.bind(i18n);
}

function renameAuditEntry() {
	return {
		action: "minecraft_profile_rename",
		entity_id: 7,
		entity_name: "RenameNew",
		entity_type: "minecraft_profile",
		presentation: {
			detail: {
				code: "minecraft_profile_renamed",
				params: {
					new_profile_name: "RenameNew",
					old_profile_name: "RenameOld",
					profile_uuid: "00000000000000000000000000000007",
					temporarily_invalidated_token_count: 1,
				},
			},
			summary: {
				code: "minecraft_profile_rename",
				params: {
					new_profile_name: "RenameNew",
					old_profile_name: "RenameOld",
				},
			},
			target: {
				code: "minecraft_profile",
				params: {
					id: 7,
					name: "RenameNew",
				},
			},
		},
	} satisfies AuditEntrySample;
}

function generatedSchemaValues(schemaName: "AuditAction" | "AuditEntityType") {
	const generatedTypesPath = resolve(
		process.cwd(),
		"src/types/api.generated.ts",
	);
	const generatedTypes = readFileSync(generatedTypesPath, "utf8");
	const match = generatedTypes.match(new RegExp(`${schemaName}: ([^;]+);`));
	if (!match) {
		throw new Error(`Missing generated schema ${schemaName}`);
	}
	return Array.from(match[1].matchAll(/"([^"]+)"/g), (item) => item[1]);
}

describe("audit i18n helpers", () => {
	it("has localized labels for all generated audit actions and entities", async () => {
		const auditActions = generatedSchemaValues("AuditAction");
		const auditEntityTypes = generatedSchemaValues("AuditEntityType");

		for (const language of ["en-US", "zh-CN"] as const) {
			const auditResources = resources[language].frontend.admin.audit;

			for (const action of auditActions) {
				expect(
					Object.hasOwn(auditResources.action, action),
					`${language} action ${action}`,
				).toBe(true);
			}

			for (const entityType of auditEntityTypes) {
				expect(
					Object.hasOwn(auditResources.entity, entityType),
					`${language} entity ${entityType}`,
				).toBe(true);
			}
		}
	});

	it("formats passkey and invitation audit labels", async () => {
		const en = await tFor("en-US");
		const zh = await tFor("zh-CN");

		expect(formatAuditAction(en, "user_passkey_login")).toBe("Passkey login");
		expect(formatAuditAction(en, "user_passkey_register")).toBe(
			"Registered passkey",
		);
		expect(formatAuditAction(en, "admin_create_invitation")).toBe(
			"Created invitation",
		);
		expect(formatAuditEntityType(en, "passkey")).toBe("Passkey");
		expect(formatAuditEntityType(en, "invitation")).toBe("Invitation");

		expect(formatAuditAction(zh, "user_passkey_login")).toBe("Passkey 登录");
		expect(formatAuditAction(zh, "user_passkey_register")).toBe("注册 Passkey");
		expect(formatAuditAction(zh, "admin_create_invitation")).toBe("创建邀请");
		expect(formatAuditEntityType(zh, "passkey")).toBe("Passkey");
		expect(formatAuditEntityType(zh, "invitation")).toBe("邀请");
	});

	it("formats Yggdrasil session forwarding audit entries", async () => {
		const en = await tFor("en-US");
		const zh = await tFor("zh-CN");
		const entry = {
			action: "yggdrasil_session_forward_check",
			entity_id: 3,
			entity_name: "Yggdrasil compatible upstream",
			entity_type: "yggdrasil_session",
			presentation: {
				detail: {
					code: "yggdrasil_session_forward_checked",
					params: {
						result: "matched",
						texture_forward_enabled: true,
						upstream_name: "Yggdrasil compatible upstream",
						username: "AptS_1548",
					},
				},
			},
		} satisfies AuditEntrySample;

		expect(
			formatAuditAction(en, "admin_update_yggdrasil_session_forward_server"),
		).toBe("Updated Yggdrasil session forwarding server");
		expect(formatAuditAction(zh, "yggdrasil_session_forward_check")).toBe(
			"Yggdrasil 会话转发检查",
		);
		expect(formatAuditDetail(en, entry)).toBe(
			"Checked Yggdrasil compatible upstream, user AptS_1548, result matched, texture forwarding true",
		);
		expect(formatAuditDetail(zh, entry)).toBe(
			"检查 Yggdrasil compatible upstream，用户 AptS_1548，结果 matched，材质转发 true",
		);
		expect(
			getAuditActionBadgeClass("yggdrasil_session_forward_check"),
		).toContain("amber");
		expect(
			getAuditActionBadgeClass("admin_update_yggdrasil_session_forward_server"),
		).toContain("sky");
	});

	it("formats Minecraft profile rename audit entries in Chinese", async () => {
		const t = await tFor("zh-CN");
		const entry = renameAuditEntry();

		expect(formatAuditSummary(t, entry)).toBe("重命名 Minecraft 角色档案");
		expect(formatAuditTarget(t, entry)).toBe("RenameNew · Minecraft 角色档案");
		expect(formatAuditDetail(t, entry)).toBe(
			"已将 RenameOld 改名为 RenameNew，临时失效 1 个 token",
		);
	});

	it("formats Minecraft profile rename audit entries in English", async () => {
		const t = await tFor("en-US");
		const entry = renameAuditEntry();

		expect(formatAuditSummary(t, entry)).toBe("Renamed Minecraft profile");
		expect(formatAuditTarget(t, entry)).toBe("RenameNew · Minecraft profile");
		expect(formatAuditDetail(t, entry)).toBe(
			"Renamed RenameOld to RenameNew; temporarily invalidated 1 token(s)",
		);
	});

	it("falls back to the action translation when a summary key is missing", async () => {
		const t = await tFor("zh-CN");
		const entry = {
			...renameAuditEntry(),
			presentation: {
				detail: { code: "unknown_profile_detail" },
				summary: { code: "unknown_profile_action" },
			},
		} satisfies AuditEntrySample;

		expect(formatAuditSummary(t, entry)).toBe("重命名 Minecraft 角色档案");
		expect(formatAuditDetail(t, entry)).toBeUndefined();
	});

	it("does not render unresolved audit presentation placeholders", async () => {
		const t = await tFor("zh-CN");
		const entry = {
			action: "admin_update_external_auth_provider",
			entity_id: 9,
			entity_name: "example",
			entity_type: "external_auth_provider",
			presentation: {
				detail: {
					code: "external_auth_provider_changed",
					params: {
						enabled: true,
						key: "example",
					},
				},
			},
		} satisfies AuditEntrySample;

		expect(formatAuditDetail(t, entry)).toBe("example，启用 true");
	});
});

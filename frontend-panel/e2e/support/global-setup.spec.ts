import { test } from "@playwright/test";
import {
	ensureAccountViaApi,
	ensureInitializedViaApi,
	loginViaApi,
	seedAccountStates,
} from "./api";
import {
	ADMIN,
	e2eBaseURL,
	OPERATOR,
	PLAYER,
	STORAGE_STATE_FILES,
} from "./fixtures";

const OPERATOR_SCOPES = [
	"com.astercosm.yggdrasil.permission.admin.users",
] as const;

/**
 * Setup project entry: runs once before the browser project and prepares the
 * shared runtime state every other spec relies on:
 *
 * 1. initializes the instance (first admin account) when needed,
 * 2. makes sure the well-known player and operator accounts exist,
 * 3. logs each role in through the API and persists its storageState.
 *
 * No browser is launched here; everything is API-driven so the browser suite
 * starts with warm sessions instead of repeating UI logins per spec.
 */
test("seed instance accounts and storage states", async ({ request }) => {
	await ensureInitializedViaApi(request, e2eBaseURL());

	await loginViaApi(request, ADMIN.username, ADMIN.password);
	await ensureAccountViaApi(request, {
		email: PLAYER.email,
		password: PLAYER.password,
		role: "user",
		username: PLAYER.username,
	});
	await ensureAccountViaApi(request, {
		email: OPERATOR.email,
		password: OPERATOR.password,
		operatorScopes: [...OPERATOR_SCOPES],
		role: "operator",
		username: OPERATOR.username,
	});

	await seedAccountStates(e2eBaseURL(), [
		{
			identifier: ADMIN.username,
			password: ADMIN.password,
			path: STORAGE_STATE_FILES.admin,
		},
		{
			identifier: PLAYER.username,
			password: PLAYER.password,
			path: STORAGE_STATE_FILES.player,
		},
		{
			identifier: OPERATOR.username,
			password: OPERATOR.password,
			path: STORAGE_STATE_FILES.operator,
		},
	]);
});

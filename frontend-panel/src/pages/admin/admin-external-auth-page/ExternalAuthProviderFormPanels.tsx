import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthKind,
	ExternalAuthProviderKindInfo,
} from "@/types/api";
import {
	callbackUrl,
	defaultScopesForKind,
	type ExternalAuthProviderFieldChange,
	type ExternalAuthProviderFormData,
	ExternalAuthProviderIcon,
	kindDescription,
	kindDisplayName,
	MICROSOFT_CUSTOM_TENANT_MODE,
	MICROSOFT_TENANT_PRESETS,
	type MicrosoftTenantMode,
	providerUsesFixedConnection,
	STANDARD_CLAIMS,
	shouldShowIssuerUrl,
	shouldShowManualEndpoints,
} from "./shared";

export function ProviderKindPanel({
	form,
	onCreateStepChange,
	onProviderKindChange,
	providerKinds,
}: {
	form: ExternalAuthProviderFormData;
	onCreateStepChange: (step: number) => void;
	onProviderKindChange: (kind: ExternalAuthKind) => void;
	providerKinds: ExternalAuthProviderKindInfo[];
}) {
	const { t } = useTranslation();

	return (
		<div className="grid gap-3 md:grid-cols-2">
			{providerKinds.map((kind) => (
				<button
					key={kind.kind}
					type="button"
					aria-pressed={form.providerKind === kind.kind}
					onClick={() => {
						onProviderKindChange(kind.kind);
						onCreateStepChange(1);
					}}
					className={cn(
						"rounded-lg border p-4 text-left transition hover:border-primary/40 hover:bg-muted/20 focus-visible:ring-3 focus-visible:ring-ring/30 focus-visible:outline-none",
						form.providerKind === kind.kind
							? "border-primary/40 bg-primary/5"
							: "border-border/70 bg-background",
					)}
				>
					<div className="flex items-start gap-4">
						<div className="flex size-14 shrink-0 items-center justify-center rounded-2xl bg-white shadow-sm ring-1 ring-black/5">
							<ExternalAuthProviderIcon
								kind={kind.kind}
								className="max-h-9 max-w-9"
							/>
						</div>
						<div className="min-w-0 flex-1">
							<div className="flex flex-wrap items-center gap-2">
								<p className="text-base font-semibold">
									{kindDisplayName(t, kind.kind, providerKinds)}
								</p>
							</div>
							<p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
								{kindDescription(t, kind)}
							</p>
						</div>
					</div>
				</button>
			))}
		</div>
	);
}

export function ProviderFormFields({
	createStepTouched,
	form,
	onCopyCallbackUrl,
	onFieldChange,
	onTestConnection,
	provider,
	selectedKind,
	testDisabled,
	testResult,
	testing,
}: {
	createStepTouched: boolean;
	form: ExternalAuthProviderFormData;
	onCopyCallbackUrl: (value: string) => void;
	onFieldChange: ExternalAuthProviderFieldChange;
	onTestConnection: () => void;
	provider: AdminExternalAuthProviderInfo | null;
	selectedKind: ExternalAuthProviderKindInfo | null;
	testDisabled: boolean;
	testResult: string | null;
	testing: boolean;
}) {
	const { t } = useTranslation();
	const fixedConnection = providerUsesFixedConnection(
		form.providerKind,
		selectedKind,
	);
	const showIssuerUrl = shouldShowIssuerUrl(selectedKind);
	const showManualEndpointFields = shouldShowManualEndpoints(selectedKind);
	const isMicrosoft = form.providerKind === "microsoft";
	const microsoftTenantOptions = [
		...MICROSOFT_TENANT_PRESETS.map((value) => ({
			label: t(`admin.externalAuth.microsoftTenant.${value}`),
			value,
		})),
		{
			label: t("admin.externalAuth.microsoftTenant.custom"),
			value: MICROSOFT_CUSTOM_TENANT_MODE,
		},
	];

	return (
		<section className="rounded-lg border border-border/70 bg-background/70 p-5">
			<div className="space-y-1">
				<h3 className="text-sm font-semibold">
					{t("admin.externalAuth.dialog.connectionTitle")}
				</h3>
				<p className="text-sm text-muted-foreground">
					{t("admin.externalAuth.dialog.connectionDesc")}
				</p>
			</div>
			<div className="mt-4 grid gap-4 md:grid-cols-2">
				<Field label={t("admin.externalAuth.displayName")} required>
					<Input
						value={form.displayName}
						aria-invalid={
							createStepTouched && !form.displayName.trim() ? true : undefined
						}
						onChange={(event) =>
							onFieldChange("displayName", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.externalAuth.iconUrl")}>
					<Input
						value={form.iconUrl}
						placeholder="https://example.com/logo.svg"
						onChange={(event) => onFieldChange("iconUrl", event.target.value)}
					/>
					<p className="text-xs leading-5 text-muted-foreground">
						{t("admin.externalAuth.iconUrlHint")}
					</p>
				</Field>
				{isMicrosoft ? (
					<Field
						label={t("admin.externalAuth.microsoftTenant.label")}
						className="md:col-span-2"
					>
						<Select
							value={form.microsoftTenantMode}
							onValueChange={(value) => {
								const mode = value as MicrosoftTenantMode;
								onFieldChange("microsoftTenantMode", mode);
								onFieldChange(
									"microsoftTenant",
									mode === MICROSOFT_CUSTOM_TENANT_MODE ? "" : mode,
								);
							}}
						>
							<SelectTrigger>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{microsoftTenantOptions.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										{option.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<p className="text-xs leading-5 text-muted-foreground">
							{t("admin.externalAuth.microsoftTenant.hint")}
						</p>
					</Field>
				) : null}
				{isMicrosoft &&
				form.microsoftTenantMode === MICROSOFT_CUSTOM_TENANT_MODE ? (
					<Field
						label={t("admin.externalAuth.microsoftTenant.customLabel")}
						className="md:col-span-2"
						required
					>
						<Input
							value={form.microsoftTenant}
							placeholder="11111111-2222-3333-4444-555555555555"
							aria-invalid={
								createStepTouched && !form.microsoftTenant.trim()
									? true
									: undefined
							}
							onChange={(event) =>
								onFieldChange("microsoftTenant", event.target.value)
							}
						/>
					</Field>
				) : null}
				<Field label={t("admin.externalAuth.clientId")} required>
					<Input
						value={form.clientId}
						aria-invalid={
							createStepTouched && !form.clientId.trim() ? true : undefined
						}
						onChange={(event) => onFieldChange("clientId", event.target.value)}
					/>
				</Field>
				<Field label={t("admin.externalAuth.clientSecret")}>
					<Input
						type="password"
						value={form.clientSecret}
						placeholder={
							provider?.client_secret_configured
								? t("admin.externalAuth.secretKeepPlaceholder")
								: ""
						}
						onChange={(event) =>
							onFieldChange("clientSecret", event.target.value)
						}
					/>
					<p className="text-xs leading-5 text-muted-foreground">
						{provider?.client_secret_configured
							? t("admin.externalAuth.secretKeepHint")
							: t("admin.externalAuth.secretHint")}
					</p>
				</Field>
				{fixedConnection ? (
					<div className="rounded-lg border border-border/70 bg-muted/25 p-4 md:col-span-2">
						<p className="text-sm font-medium">
							{t("admin.externalAuth.fixedConnection.title")}
						</p>
						<p className="mt-1 text-xs leading-5 text-muted-foreground">
							{t(
								`admin.externalAuth.fixedConnection.${form.providerKind}.description`,
							)}
						</p>
					</div>
				) : null}
				{showIssuerUrl ? (
					<Field label={t("admin.externalAuth.issuerUrl")} required>
						<Input
							value={form.issuerUrl}
							placeholder="https://id.example.com"
							aria-invalid={
								createStepTouched &&
								selectedKind?.issuer_url_required &&
								!form.issuerUrl.trim()
									? true
									: undefined
							}
							onChange={(event) =>
								onFieldChange("issuerUrl", event.target.value)
							}
						/>
					</Field>
				) : null}
				{showManualEndpointFields ? (
					<>
						<Field label={t("admin.externalAuth.authorizationUrl")} required>
							<Input
								value={form.authorizationUrl}
								placeholder="https://id.example.com/oauth/authorize"
								aria-invalid={
									createStepTouched &&
									selectedKind?.authorization_url_required &&
									!form.authorizationUrl.trim()
										? true
										: undefined
								}
								onChange={(event) =>
									onFieldChange("authorizationUrl", event.target.value)
								}
							/>
						</Field>
						<Field label={t("admin.externalAuth.tokenUrl")} required>
							<Input
								value={form.tokenUrl}
								placeholder="https://id.example.com/oauth/token"
								aria-invalid={
									createStepTouched &&
									selectedKind?.token_url_required &&
									!form.tokenUrl.trim()
										? true
										: undefined
								}
								onChange={(event) =>
									onFieldChange("tokenUrl", event.target.value)
								}
							/>
						</Field>
						<Field label={t("admin.externalAuth.userinfoUrl")} required>
							<Input
								value={form.userinfoUrl}
								placeholder="https://id.example.com/oauth/userinfo"
								aria-invalid={
									createStepTouched &&
									selectedKind?.userinfo_url_required &&
									!form.userinfoUrl.trim()
										? true
										: undefined
								}
								onChange={(event) =>
									onFieldChange("userinfoUrl", event.target.value)
								}
							/>
						</Field>
					</>
				) : null}
				{fixedConnection ? null : (
					<Field
						label={t("admin.externalAuth.scopes")}
						className="md:col-span-2"
					>
						<Textarea
							value={form.scopes}
							rows={2}
							onChange={(event) => onFieldChange("scopes", event.target.value)}
						/>
					</Field>
				)}
				<Field
					label={t("admin.externalAuth.claims.subject")}
					className="md:col-span-2"
				>
					<Input
						value={form.subjectClaim}
						placeholder={STANDARD_CLAIMS.subjectClaim}
						onChange={(event) =>
							onFieldChange("subjectClaim", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.externalAuth.claims.username")}>
					<Input
						value={form.usernameClaim}
						placeholder={STANDARD_CLAIMS.usernameClaim}
						onChange={(event) =>
							onFieldChange("usernameClaim", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.externalAuth.claims.displayName")}>
					<Input
						value={form.displayNameClaim}
						placeholder={STANDARD_CLAIMS.displayNameClaim}
						onChange={(event) =>
							onFieldChange("displayNameClaim", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.externalAuth.claims.email")}>
					<Input
						value={form.emailClaim}
						placeholder={STANDARD_CLAIMS.emailClaim}
						onChange={(event) =>
							onFieldChange("emailClaim", event.target.value)
						}
					/>
				</Field>
				{selectedKind?.supports_email_verified_claim ? (
					<Field label={t("admin.externalAuth.claims.emailVerified")}>
						<Input
							value={form.emailVerifiedClaim}
							placeholder={STANDARD_CLAIMS.emailVerifiedClaim}
							onChange={(event) =>
								onFieldChange("emailVerifiedClaim", event.target.value)
							}
						/>
					</Field>
				) : null}
				<Field label={t("admin.externalAuth.claims.groups")}>
					<Input
						value={form.groupsClaim}
						placeholder={STANDARD_CLAIMS.groupsClaim}
						onChange={(event) =>
							onFieldChange("groupsClaim", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.externalAuth.claims.avatar")}>
					<Input
						value={form.avatarUrlClaim}
						placeholder={STANDARD_CLAIMS.avatarUrlClaim}
						onChange={(event) =>
							onFieldChange("avatarUrlClaim", event.target.value)
						}
					/>
				</Field>
				{provider ? (
					<Field
						label={t("admin.externalAuth.callbackUrl")}
						className="md:col-span-2"
					>
						<div className="flex min-w-0 gap-2">
							<Input readOnly value={callbackUrl(provider)} />
							<Button
								type="button"
								variant="outline"
								size="icon"
								onClick={() => onCopyCallbackUrl(callbackUrl(provider))}
								aria-label={t("admin.externalAuth.copyCallback")}
							>
								<Icon name="Copy" className="size-4" />
							</Button>
						</div>
					</Field>
				) : null}
				<div className="flex min-w-0 flex-wrap items-center gap-3 md:col-span-2">
					<Button
						type="button"
						variant="outline"
						disabled={testDisabled}
						onClick={onTestConnection}
					>
						<Icon
							name={testing ? "Spinner" : "WifiHigh"}
							className={cn("mr-2 size-4", testing && "animate-spin")}
						/>
						{t("admin.externalAuth.test")}
					</Button>
					{testResult ? (
						<p className="min-w-0 flex-1 text-sm text-emerald-700 dark:text-emerald-300">
							{testResult}
						</p>
					) : (
						<p className="min-w-0 flex-1 text-sm text-muted-foreground">
							{t("admin.externalAuth.testHint")}
						</p>
					)}
				</div>
			</div>
		</section>
	);
}

export function AccessPanel({
	form,
	onFieldChange,
}: {
	form: ExternalAuthProviderFormData;
	onFieldChange: ExternalAuthProviderFieldChange;
}) {
	const { t } = useTranslation();

	return (
		<section className="rounded-lg border border-border/70 bg-background/70 p-5">
			<h3 className="text-sm font-semibold">
				{t("admin.externalAuth.dialog.accessTitle")}
			</h3>
			<div className="mt-4 space-y-4">
				<div className="flex items-start gap-3">
					<Switch
						id="external-auth-enabled"
						checked={form.enabled}
						onCheckedChange={(value) => onFieldChange("enabled", value)}
					/>
					<div className="space-y-1">
						<Label htmlFor="external-auth-enabled">
							{t("admin.externalAuth.enabled")}
						</Label>
						<p className="text-sm text-muted-foreground">
							{t("admin.externalAuth.enabledDesc")}
						</p>
					</div>
				</div>
				<div className="flex items-start gap-3">
					<Switch
						id="external-auth-require-email-verified"
						checked={form.requireEmailVerified}
						onCheckedChange={(value) =>
							onFieldChange("requireEmailVerified", value)
						}
					/>
					<div className="space-y-1">
						<Label htmlFor="external-auth-require-email-verified">
							{t("admin.externalAuth.requireEmailVerified")}
						</Label>
						<p className="text-sm text-muted-foreground">
							{t("admin.externalAuth.requireEmailVerifiedDesc")}
						</p>
					</div>
				</div>
				<div className="flex items-start gap-3">
					<Switch
						id="external-auth-auto-link"
						checked={form.autoLinkVerifiedEmailEnabled}
						onCheckedChange={(value) =>
							onFieldChange("autoLinkVerifiedEmailEnabled", value)
						}
					/>
					<div className="space-y-1">
						<Label htmlFor="external-auth-auto-link">
							{t("admin.externalAuth.autoLinkVerifiedEmail")}
						</Label>
						<p className="text-sm text-muted-foreground">
							{t("admin.externalAuth.autoLinkVerifiedEmailDesc")}
						</p>
					</div>
				</div>
				<div className="flex items-start gap-3">
					<Switch
						id="external-auth-auto-provision"
						checked={form.autoProvisionEnabled}
						onCheckedChange={(value) =>
							onFieldChange("autoProvisionEnabled", value)
						}
					/>
					<div className="space-y-1">
						<Label htmlFor="external-auth-auto-provision">
							{t("admin.externalAuth.autoProvision")}
						</Label>
						<p className="text-sm text-muted-foreground">
							{t("admin.externalAuth.autoProvisionDesc")}
						</p>
					</div>
				</div>
				<Field label={t("admin.externalAuth.allowedDomains")}>
					<Textarea
						value={form.allowedDomains}
						rows={3}
						placeholder="example.com, example.org"
						onChange={(event) =>
							onFieldChange("allowedDomains", event.target.value)
						}
					/>
					<p className="text-sm text-muted-foreground">
						{t("admin.externalAuth.allowedDomainsDesc")}
					</p>
				</Field>
			</div>
		</section>
	);
}

export function ProviderSummaryPanel({
	form,
	provider,
	providerKinds,
	selectedKind,
}: {
	form: ExternalAuthProviderFormData;
	provider: AdminExternalAuthProviderInfo | null;
	providerKinds: ExternalAuthProviderKindInfo[];
	selectedKind: ExternalAuthProviderKindInfo | null;
}) {
	const { t } = useTranslation();
	const fixedConnection = providerUsesFixedConnection(
		form.providerKind,
		selectedKind,
	);
	const primaryEndpoint = fixedConnection
		? t(`admin.externalAuth.fixedConnection.${form.providerKind}.summary`)
		: form.issuerUrl ||
			form.authorizationUrl ||
			form.userinfoUrl ||
			form.tokenUrl ||
			t("admin.externalAuth.table.noEndpoint");

	return (
		<aside className="h-fit rounded-lg border border-border/70 bg-muted/20 p-5">
			<h3 className="text-sm font-semibold">
				{t("admin.externalAuth.dialog.summaryTitle")}
			</h3>
			<dl className="mt-4 space-y-3 text-sm">
				<SummaryItem label={t("admin.externalAuth.kindLabel")}>
					{kindDisplayName(t, form.providerKind, providerKinds)}
				</SummaryItem>
				<SummaryItem label={t("admin.externalAuth.key")}>
					{form.key || t("admin.externalAuth.generatedKey")}
				</SummaryItem>
				<SummaryItem label={t("admin.externalAuth.iconUrl")}>
					{form.iconUrl.trim() || t("admin.externalAuth.defaultIcon")}
				</SummaryItem>
				<SummaryItem label={t("admin.externalAuth.primaryEndpoint")}>
					{primaryEndpoint}
				</SummaryItem>
				{fixedConnection ? null : (
					<SummaryItem label={t("admin.externalAuth.scopes")}>
						{form.scopes.trim() || defaultScopesForKind(selectedKind)}
					</SummaryItem>
				)}
				<SummaryItem label={t("admin.externalAuth.status")}>
					{form.enabled
						? t("admin.externalAuth.enabled")
						: t("admin.externalAuth.disabled")}
				</SummaryItem>
				{provider ? (
					<SummaryItem label={t("admin.externalAuth.callbackUrl")}>
						<span className="break-all font-mono">{callbackUrl(provider)}</span>
					</SummaryItem>
				) : null}
			</dl>
		</aside>
	);
}

function SummaryItem({
	children,
	label,
}: {
	children: React.ReactNode;
	label: string;
}) {
	return (
		<div>
			<dt className="text-xs text-muted-foreground">{label}</dt>
			<dd className="mt-1 break-words text-xs">{children}</dd>
		</div>
	);
}

function Field({
	children,
	className,
	label,
	required,
}: {
	children: React.ReactNode;
	className?: string;
	label: string;
	required?: boolean;
}) {
	return (
		<div className={cn("space-y-2", className)}>
			<Label>
				{label}
				{required ? <span className="text-destructive"> *</span> : null}
			</Label>
			{children}
		</div>
	);
}

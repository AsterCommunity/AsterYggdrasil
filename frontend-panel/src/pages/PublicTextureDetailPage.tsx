import { type FormEvent, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { buttonVariants } from "@/components/ui/buttonVariants";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { PublicTextureHeaderActions } from "@/components/yggdrasil/PublicTextureHeaderActions";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePublicSession } from "@/hooks/usePublicSession";
import { formatBytes } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import {
	formatTextureKind,
	PublicTextureCopyDialog,
	PublicTextureDetail,
	TextureLibraryDisabledPanel,
} from "@/pages/PublicTextureLibraryPage";
import { publicPaths } from "@/routes/routePaths";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	MinecraftTextureReportReason,
	PublicTextureLibraryTextureMetadata,
} from "@/types/api";

const TEXTURE_REPORT_REASONS = [
	"inappropriate",
	"offensive",
	"copyright",
	"misleading",
	"broken",
	"spam",
	"other",
] as const satisfies readonly MinecraftTextureReportReason[];

export default function PublicTextureDetailPage() {
	const { textureId } = useParams();
	const navigate = useNavigate();
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const textureLibraryEnabled = useFrontendConfigStore(
		(state) => state.textureLibrary.enabled,
	);
	const { isAuthenticated, logout, user } = usePublicSession();
	const [texture, setTexture] =
		useState<PublicTextureLibraryTextureMetadata | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [copyDialogOpen, setCopyDialogOpen] = useState(false);
	const [reportDialogOpen, setReportDialogOpen] = useState(false);
	const parsedTextureId = Number(textureId);
	const validTextureId =
		Number.isSafeInteger(parsedTextureId) && parsedTextureId > 0
			? parsedTextureId
			: null;
	const serverName = branding.title || t("home.titleFallback");

	usePageTitle(texture?.name ?? t("library.detailTitle"));

	const loadTexture = useCallback(async () => {
		if (!textureLibraryEnabled) {
			setTexture(null);
			setError(t("library.disabledDescription"));
			setLoading(false);
			return;
		}
		if (!validTextureId) {
			setTexture(null);
			setError(t("library.detailNotFound"));
			setLoading(false);
			return;
		}
		setLoading(true);
		setError(null);
		try {
			setTexture(
				await yggdrasilService.getPublicTextureLibraryTexture(validTextureId),
			);
		} catch (nextError) {
			setTexture(null);
			setError(formatUnknownError(nextError));
		} finally {
			setLoading(false);
		}
	}, [t, textureLibraryEnabled, validTextureId]);

	useEffect(() => {
		void loadTexture();
	}, [loadTexture]);

	return (
		<PublicEntryShell
			branding={branding}
			title={serverName}
			tagline={t("brand.tagline")}
			variant="home"
			hideLanguageOnMobile
			headerActions={
				isAuthenticated && user ? (
					<PublicTextureHeaderActions
						isAuthenticated={isAuthenticated}
						user={user}
						onLogout={() => void logout()}
					/>
				) : (
					<Link
						to={publicPaths.login}
						className={cn(
							buttonVariants({ variant: "default", size: "sm" }),
							"h-10 rounded-lg border-emerald-300/24 bg-emerald-500 px-3 text-white shadow-lg shadow-emerald-950/35 hover:bg-emerald-400 sm:px-4",
						)}
					>
						<Icon name="SignIn" className="size-4" />
						<span className="hidden sm:inline">{t("home.loginRegister")}</span>
					</Link>
				)
			}
			footer={<AppFooter />}
		>
			<main className="relative z-10 min-w-0 flex-1">
				<div className="mx-auto grid w-full max-w-[92rem] gap-5 px-4 pt-6 pb-10 sm:px-8 lg:px-12">
					{textureLibraryEnabled ? (
						<div>
							<Link
								to={publicPaths.textureLibrary}
								className="inline-flex items-center gap-2 rounded-md text-sm font-medium text-slate-700 transition hover:text-[#102118] focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35 dark:text-slate-300 dark:hover:text-white"
							>
								<Icon name="ArrowLeft" className="size-4" />
								{t("library.backToLibrary")}
							</Link>
						</div>
					) : null}

					{loading ? (
						<div className="grid gap-4 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
							<Skeleton className="h-[28rem] rounded-xl" />
							<Skeleton className="h-[28rem] rounded-xl" />
						</div>
					) : !textureLibraryEnabled ? (
						<TextureLibraryDisabledPanel />
					) : error || !texture ? (
						<section className="grid min-h-[24rem] place-items-center rounded-xl border border-black/10 bg-white/76 px-4 text-center shadow-2xl shadow-emerald-950/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/25">
							<div className="max-w-md">
								<h1 className="text-xl font-semibold tracking-normal">
									{t("library.detailUnavailableTitle")}
								</h1>
								<p className="mt-2 text-sm leading-6 text-muted-foreground">
									{error ?? t("library.detailNotFound")}
								</p>
								<div className="mt-5 flex flex-wrap justify-center gap-2">
									<Button type="button" onClick={() => void loadTexture()}>
										<Icon name="RefreshCw" className="size-4" />
										{t("common.refresh")}
									</Button>
									<Link
										to={publicPaths.textureLibrary}
										className={buttonVariants({ variant: "outline" })}
									>
										{t("library.backToLibrary")}
									</Link>
								</div>
							</div>
						</section>
					) : (
						<>
							<header className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
								<div className="min-w-0">
									<Badge className="rounded-full border-emerald-700/20 bg-emerald-600/12 px-3 py-1 text-emerald-800 shadow-lg shadow-black/10 dark:border-emerald-300/24 dark:bg-emerald-400/14 dark:text-emerald-100">
										<Icon name="FileImage" className="size-3.5" />
										{formatTextureKind(texture, t)}
									</Badge>
									<h1 className="mt-4 max-w-4xl text-balance font-black text-4xl leading-none tracking-normal text-[#102118] sm:text-6xl dark:text-white">
										{texture.name}
									</h1>
									<div className="mt-4 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-slate-700 dark:text-slate-300">
										<span>
											{texture.uploader?.name ?? t("library.unknownUploader")}
										</span>
										<span>
											{texture.width}x{texture.height}
										</span>
										<span>{formatBytes(texture.file_size)}</span>
									</div>
								</div>
								<div className="flex flex-wrap items-center justify-start gap-2 lg:justify-end">
									<Button
										type="button"
										variant="outline"
										onClick={() => {
											if (!isAuthenticated) {
												navigate(publicPaths.login);
												return;
											}
											setReportDialogOpen(true);
										}}
									>
										<Icon name="Flag" className="size-4" />
										{t("library.reportAction")}
									</Button>
									<Button type="button" onClick={() => setCopyDialogOpen(true)}>
										<Icon name="Copy" className="size-4" />
										{t("library.copyAction")}
									</Button>
								</div>
							</header>

							<section>
								<div className="rounded-xl border border-black/10 bg-white/76 p-4 shadow-2xl shadow-emerald-950/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/25">
									<PublicTextureDetail texture={texture} />
								</div>
							</section>
						</>
					)}
				</div>

				<PublicTextureCopyDialog
					open={copyDialogOpen}
					texture={texture}
					onOpenChange={setCopyDialogOpen}
				/>
				<PublicTextureReportDialog
					open={reportDialogOpen}
					texture={texture}
					onOpenChange={setReportDialogOpen}
				/>
			</main>
		</PublicEntryShell>
	);
}

function PublicTextureReportDialog({
	onOpenChange,
	open,
	texture,
}: {
	onOpenChange: (open: boolean) => void;
	open: boolean;
	texture: PublicTextureLibraryTextureMetadata | null;
}) {
	const { t } = useTranslation();
	const [reason, setReason] =
		useState<MinecraftTextureReportReason>("inappropriate");
	const [message, setMessage] = useState("");
	const [submitting, setSubmitting] = useState(false);

	useEffect(() => {
		if (open) {
			setReason("inappropriate");
			setMessage("");
		}
	}, [open]);

	async function submitReport(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!texture) return;
		setSubmitting(true);
		try {
			await yggdrasilService.createTextureReport(texture.id, {
				reason,
				message: message.trim() || null,
			});
			toast.success(t("library.reportSuccess"));
			onOpenChange(false);
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			setSubmitting(false);
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent keepMounted className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>{t("library.reportDialogTitle")}</DialogTitle>
					<DialogDescription>
						{texture
							? t("library.reportDialogDescription", { name: texture.name })
							: t("library.detailFallback")}
					</DialogDescription>
				</DialogHeader>

				{texture ? (
					<form
						id="public-texture-report-form"
						className="space-y-4"
						onSubmit={(event) => void submitReport(event)}
					>
						<div className="grid gap-2">
							<Label htmlFor="public-texture-report-reason">
								{t("library.reportReasonLabel")}
							</Label>
							<Select
								value={reason}
								disabled={submitting}
								onValueChange={(value) =>
									setReason(value as MinecraftTextureReportReason)
								}
							>
								<SelectTrigger id="public-texture-report-reason">
									<span data-slot="select-value">
										{t(`library.reportReason.${reason}`)}
									</span>
								</SelectTrigger>
								<SelectContent>
									{TEXTURE_REPORT_REASONS.map((reason) => (
										<SelectItem key={reason} value={reason}>
											{t(`library.reportReason.${reason}`)}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
						<div className="grid gap-2">
							<Label htmlFor="public-texture-report-message">
								{t("library.reportMessageLabel")}
							</Label>
							<Textarea
								id="public-texture-report-message"
								value={message}
								maxLength={1000}
								disabled={submitting}
								placeholder={t("library.reportMessagePlaceholder")}
								onChange={(event) => setMessage(event.currentTarget.value)}
							/>
							<p className="text-xs leading-5 text-muted-foreground">
								{t("library.reportMessageHelp")}
							</p>
						</div>
					</form>
				) : null}

				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						disabled={submitting}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button
						type="submit"
						form="public-texture-report-form"
						disabled={!texture || submitting}
					>
						<Icon
							name={submitting ? "Spinner" : "Flag"}
							className={cn("size-4", submitting && "animate-spin")}
						/>
						{submitting
							? t("library.reportSubmitting")
							: t("library.reportSubmitAction")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

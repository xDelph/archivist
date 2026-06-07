import { IdentityAvatar } from "@/components/identity-avatar";
import { SectionCard } from "@/components/section-card";
import { Button, buttonVariants } from "@/components/ui/button";
import {
	anonymizeCurrentUser,
	deAnonymizeCurrentUser,
	logoutCurrentUser,
} from "@/lib/api";
import { clearCachedCurrentUser } from "@/lib/auth-cache";
import { formatSlackTimestamp } from "@/lib/format";
import { authQueries } from "@/lib/queries";
import { displayAuthorName } from "@/lib/thread-display";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useNavigate } from "@tanstack/react-router";
import { cn } from "@/lib/utils";
import { LogOut, Shield } from "lucide-react";

export function AccountPage() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const userQuery = useQuery(authQueries.me());
	const logoutMutation = useMutation({
		mutationFn: logoutCurrentUser,
		onSuccess: async () => {
			clearCachedCurrentUser();
			queryClient.removeQueries({ queryKey: ["auth"] });
			await navigate({ to: "/sign-in" });
		},
	});
	const anonymizeMutation = useMutation({
		mutationFn: anonymizeCurrentUser,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["auth"] });
		},
	});
	const deAnonymizeMutation = useMutation({
		mutationFn: deAnonymizeCurrentUser,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["auth"] });
		},
	});

	const user = userQuery.data?.user;
	const isAdmin = user?.roles.includes("admin") ?? false;
	const privacyPending =
		anonymizeMutation.isPending || deAnonymizeMutation.isPending;
	const displayName = user
		? displayAuthorName(
				{
					slack_user_id: user.slack_user_id,
					display_name: user.display_name,
					avatar_url: user.avatar_url,
				},
				user.email,
			)
		: "Signed-in member";

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Account"
				title={displayName}
				description="Identity comes from Slack OIDC. The session stays in an HttpOnly cookie."
			>
				<div className="mb-5 flex items-center gap-4">
					<IdentityAvatar
						author={
							user
								? {
										slack_user_id: user.slack_user_id,
										display_name: user.display_name,
										avatar_url: user.avatar_url,
									}
								: null
						}
						fallback={user?.email ?? "Arkivist"}
						size="lg"
						className="size-14 text-lg"
					/>
					<div>
						<p className="text-base font-semibold text-(--color-text-primary)">
							{displayName}
						</p>
						<p className="text-sm text-(--color-text-muted)">
							{user?.is_anonymized
								? "Anonymous mode is active"
								: user?.email || user?.slack_user_id || "Slack workspace"}
						</p>
					</div>
				</div>

				<div className="grid gap-3 sm:grid-cols-2">
					<AccountRow
						label="Slack user"
						value={user?.slack_user_id || "Unknown"}
					/>
					<AccountRow
						label="Email"
						value={
							user?.is_anonymized
								? "Hidden while anonymous"
								: user?.email || "Not available"
						}
					/>
					<AccountRow
						label="Roles"
						value={user?.roles?.length ? user.roles.join(", ") : "Member"}
					/>
					<AccountRow
						label="Status"
						value={
							user?.is_active === false
								? "Deactivated by admin"
								: user?.is_anonymized
									? "Anonymous"
									: "Active"
						}
					/>
					<AccountRow
						label="Last checked"
						value={formatSlackTimestamp(`${Date.now() / 1_000}`)}
					/>
				</div>
				<div className="mt-5 flex flex-wrap gap-3">
					<Button
						type="button"
						variant="secondary"
						onClick={() => logoutMutation.mutate()}
						disabled={logoutMutation.isPending}
					>
						<LogOut className="size-4" />
						{logoutMutation.isPending ? "Signing out" : "Sign out"}
					</Button>
					{isAdmin ? (
						<Link
							to="/admin/users"
							className={cn(buttonVariants({ variant: "secondary" }))}
						>
							<Shield className="size-4" />
							Manage users
						</Link>
					) : null}
				</div>
			</SectionCard>

			<SectionCard
				eyebrow="Privacy"
				title="Anonymous mode"
				description="Hide your display name and avatar across Arkivist. Your Slack user ID stays attached to messages for archive integrity."
			>
				{user?.is_active === false ? (
					<p className="text-sm leading-relaxed text-(--color-text-secondary)">
						Your account was deactivated by an administrator. Contact an admin to
						restore access.
					</p>
				) : user?.is_anonymized ? (
					<div className="space-y-4">
						<p className="text-sm leading-relaxed text-(--color-text-secondary)">
							You currently appear as <strong>@anonymous</strong> everywhere in
							Arkivist. Slack profile sync is paused until you turn this off.
						</p>
						<Button
							type="button"
							variant="secondary"
							onClick={() => deAnonymizeMutation.mutate()}
							disabled={privacyPending}
						>
							{deAnonymizeMutation.isPending
								? "Restoring identity"
								: "Show my identity again"}
						</Button>
					</div>
				) : (
					<div className="space-y-4">
						<p className="text-sm leading-relaxed text-(--color-text-secondary)">
							Turn on anonymous mode to hide your name, email, and avatar. You
							can turn it off again any time while your account stays active.
						</p>
						<Button
							type="button"
							variant="secondary"
							onClick={() => anonymizeMutation.mutate()}
							disabled={privacyPending}
						>
							{anonymizeMutation.isPending
								? "Applying anonymous mode"
								: "Become @anonymous"}
						</Button>
					</div>
				)}
			</SectionCard>
		</div>
	);
}

function AccountRow({ label, value }: { label: string; value: string }) {
	return (
		<div className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
			<p className="text-[0.6rem] font-medium uppercase tracking-[0.22em] text-(--color-text-muted)">
				{label}
			</p>
			<p className="mt-1.5 text-sm text-(--color-text-primary)">{value}</p>
		</div>
	);
}

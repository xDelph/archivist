import { SectionCard } from "@/components/section-card";
import { Button } from "@/components/ui/button";
import { logoutCurrentUser } from "@/lib/api";
import { formatSlackTimestamp, initials } from "@/lib/format";
import { authQueries } from "@/lib/queries";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { LogOut } from "lucide-react";

export function AccountPage() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const userQuery = useQuery(authQueries.me());
	const logoutMutation = useMutation({
		mutationFn: logoutCurrentUser,
		onSuccess: async () => {
			queryClient.removeQueries({ queryKey: ["auth"] });
			await navigate({ to: "/sign-in" });
		},
	});

	const user = userQuery.data?.user;

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Account"
				title={user?.display_name || user?.email || "Slack identity"}
				description="Identity comes from Slack OIDC. The session stays in an HttpOnly cookie."
			>
				<div className="mb-5 flex items-center gap-4">
					{user?.avatar_url ? (
						<img
							src={user.avatar_url}
							alt=""
							className="size-14 rounded-full object-cover"
						/>
					) : (
						<span className="flex size-14 items-center justify-center rounded-full bg-(--color-accent-soft)/15 text-lg font-semibold text-(--color-accent-soft)">
							{initials(user?.display_name ?? user?.email)}
						</span>
					)}
					<div>
						<p className="text-base font-semibold text-(--color-text-primary)">
							{user?.display_name || "Signed-in member"}
						</p>
						<p className="text-sm text-(--color-text-muted)">
							{user?.email || user?.slack_user_id || "Slack workspace"}
						</p>
					</div>
				</div>

				<div className="grid gap-3 sm:grid-cols-2">
					<AccountRow
						label="Slack user"
						value={user?.slack_user_id || "Unknown"}
					/>
					<AccountRow label="Workspace" value={user?.team_id || "Unknown"} />
					<AccountRow label="Email" value={user?.email || "Not available"} />
					<AccountRow
						label="Last checked"
						value={formatSlackTimestamp(`${Date.now() / 1_000}`)}
					/>
				</div>
				<div className="mt-5">
					<Button
						type="button"
						variant="secondary"
						onClick={() => logoutMutation.mutate()}
						disabled={logoutMutation.isPending}
					>
						<LogOut className="size-4" />
						{logoutMutation.isPending ? "Signing out" : "Sign out"}
					</Button>
				</div>
			</SectionCard>

			<SectionCard
				eyebrow="Preferences"
				title="Preference controls are next"
				description="Preference management lands in a later milestone."
			>
				<p className="text-sm leading-relaxed text-(--color-text-secondary)">
					Planned follow-ups include digest timing, default catch-up windows,
					and personal saved-item preferences.
				</p>
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

import { SectionCard } from "@/components/section-card";
import { Button } from "@/components/ui/button";
import { logoutCurrentUser } from "@/lib/api";
import { currentUserQueryOptions } from "@/lib/auth";
import { formatSlackTimestamp } from "@/lib/format";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { LogOut } from "lucide-react";

export function AccountPage() {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const userQuery = useQuery(currentUserQueryOptions());
	const logoutMutation = useMutation({
		mutationFn: logoutCurrentUser,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["auth"] });
			await navigate({ to: "/sign-in" });
		},
	});

	const user = userQuery.data?.user;

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Account"
				title={user?.display_name || user?.email || "Slack identity"}
				description="Identity currently comes from Slack OIDC through `apps/api`, and the session stays in an HttpOnly cookie."
			>
				<div className="grid gap-4 sm:grid-cols-2">
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
						<LogOut className="mr-2 size-4" />
						{logoutMutation.isPending ? "Signing out" : "Sign out"}
					</Button>
				</div>
			</SectionCard>

			<SectionCard
				eyebrow="Preferences"
				title="Preference controls are next"
				description="The account route exists now for identity and sign-out. Preference management lands later in the milestone."
			>
				<p className="text-sm leading-6 text-slate-300">
					Planned follow-ups here include digest timing, default catch-up
					windows, and personal saved-item preferences.
				</p>
			</SectionCard>
		</div>
	);
}

function AccountRow({ label, value }: { label: string; value: string }) {
	return (
		<div className="rounded-[1.25rem] border border-white/10 bg-slate-950/35 p-4">
			<p className="text-[0.65rem] uppercase tracking-[0.22em] text-slate-400">
				{label}
			</p>
			<p className="mt-2 text-sm text-white">{value}</p>
		</div>
	);
}

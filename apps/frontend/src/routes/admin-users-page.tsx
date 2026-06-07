import { SectionCard } from "@/components/section-card";
import { Button, buttonVariants } from "@/components/ui/button";
import {
	type AdminUser,
	adminAnonymizeUser,
	adminDeactivateUser,
	adminDeAnonymizeUser,
	adminReactivateUser,
	fetchAdminUsers,
} from "@/lib/api";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";

export function AdminUsersPage() {
	const queryClient = useQueryClient();
	const userQuery = useQuery(authQueries.me());
	const [search, setSearch] = useState("");
	const [pendingUserId, setPendingUserId] = useState<string | null>(null);
	const usersQuery = useQuery({
		queryKey: ["admin", "users", search],
		queryFn: () => fetchAdminUsers(search),
		enabled: userQuery.data?.user.roles.includes("admin") ?? false,
	});
	const actionMutation = useMutation({
		mutationFn: async ({
			userId,
			action,
		}: {
			userId: string;
			action: AdminAction;
		}) => {
			setPendingUserId(userId);
			switch (action) {
				case "anonymize":
					return adminAnonymizeUser(userId);
				case "de-anonymize":
					return adminDeAnonymizeUser(userId);
				case "deactivate":
					return adminDeactivateUser(userId);
				case "reactivate":
					return adminReactivateUser(userId);
			}
		},
		onSettled: async () => {
			setPendingUserId(null);
			await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
		},
	});

	const isAdmin = userQuery.data?.user.roles.includes("admin") ?? false;
	const users = useMemo(() => usersQuery.data?.users ?? [], [usersQuery.data]);

	if (!isAdmin) {
		return (
			<SectionCard
				eyebrow="Admin"
				title="Admin access required"
				description="Only administrators can manage workspace users."
			>
				<Link
					to="/account"
					className={cn(buttonVariants({ variant: "secondary" }))}
				>
					Back to account
				</Link>
			</SectionCard>
		);
	}

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Admin"
				title="Workspace users"
				description="Anonymize, restore, deactivate, or reactivate Slack members. Deactivation also blocks sign-in."
			>
				<div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center">
					<input
						type="search"
						value={search}
						onChange={(event) => setSearch(event.target.value)}
						placeholder="Search by name, email, or Slack ID"
						className="w-full rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 px-3 py-2 text-sm text-(--color-text-primary) outline-none focus:border-(--color-border-accent)"
					/>
					<Link
						to="/account"
						className={cn(buttonVariants({ variant: "secondary" }))}
					>
						Back to account
					</Link>
				</div>

				{usersQuery.isPending ? (
					<p className="text-sm text-(--color-text-secondary)">
						Loading users...
					</p>
				) : users.length === 0 ? (
					<p className="text-sm text-(--color-text-secondary)">
						No users matched this search.
					</p>
				) : (
					<div className="space-y-3">
						{users.map((user) => (
							<AdminUserRow
								key={user.slack_user_id}
								user={user}
								isPending={
									actionMutation.isPending &&
									pendingUserId === user.slack_user_id
								}
								onAction={(action) =>
									actionMutation.mutate({
										userId: user.slack_user_id,
										action,
									})
								}
							/>
						))}
					</div>
				)}
			</SectionCard>
		</div>
	);
}

type AdminAction = "anonymize" | "de-anonymize" | "deactivate" | "reactivate";

function AdminUserRow({
	user,
	isPending,
	onAction,
}: {
	user: AdminUser;
	isPending: boolean;
	onAction: (action: AdminAction) => void;
}) {
	const status = !user.is_active
		? "Deactivated"
		: user.is_anonymized
			? "Anonymous"
			: "Active";

	return (
		<div className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
			<div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
				<div className="min-w-0">
					<p className="truncate text-sm font-semibold text-(--color-text-primary)">
						{user.display_name === "anonymous"
							? "@anonymous"
							: user.display_name || user.slack_user_id}
					</p>
					<p className="mt-1 text-sm text-(--color-text-muted)">
						{user.email || user.slack_user_id}
					</p>
					<p className="mt-2 text-[0.72rem] uppercase tracking-[0.18em] text-(--color-text-faint)">
						{status}
						{user.roles.length ? ` · ${user.roles.join(", ")}` : ""}
					</p>
				</div>
				<div className="flex flex-wrap gap-2">
					{user.is_active && !user.is_anonymized ? (
						<Button
							type="button"
							variant="secondary"
							disabled={isPending}
							onClick={() => onAction("anonymize")}
						>
							Anonymize
						</Button>
					) : null}
					{user.is_active && user.is_anonymized ? (
						<Button
							type="button"
							variant="secondary"
							disabled={isPending}
							onClick={() => onAction("de-anonymize")}
						>
							Restore identity
						</Button>
					) : null}
					{user.is_active ? (
						<Button
							type="button"
							variant="secondary"
							disabled={isPending}
							onClick={() => onAction("deactivate")}
						>
							Deactivate
						</Button>
					) : (
						<Button
							type="button"
							variant="secondary"
							disabled={isPending}
							onClick={() => onAction("reactivate")}
						>
							Reactivate
						</Button>
					)}
				</div>
			</div>
		</div>
	);
}

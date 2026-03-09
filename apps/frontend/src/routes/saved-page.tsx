import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";

export function SavedPage() {
	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Saved"
				title="Bookmarked threads"
				description="The route exists now so the app shell is complete. The persistence layer lands when the `saved_items` backend tasks are reached."
			>
				<EmptyState
					title="Save your first thread"
					description="Once save actions are wired, this screen will hold the threads you want to revisit without searching again."
				/>
			</SectionCard>
		</div>
	);
}

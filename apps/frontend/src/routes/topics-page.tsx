import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { Layers3 } from "lucide-react";

export function TopicsPage() {
	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Topics"
				title="Topic clustering is still ahead"
				description="The navigation slot is in place so the mobile shell is complete, but topic extraction belongs to a later milestone."
			>
				<EmptyState
					title="No topic clusters yet"
					description="Once the product starts extracting recurring themes, this route will surface them as reusable entry points into the archive."
					icon={<Layers3 className="size-5" />}
				/>
			</SectionCard>
		</div>
	);
}

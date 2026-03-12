import {
	type ThreadAuthor,
	authorAvatarTone,
	authorInitials,
	displayAuthorName,
} from "@/lib/thread-display";
import { cn } from "@/lib/utils";

interface IdentityAvatarProps {
	author?: ThreadAuthor | null;
	fallback?: string | null;
	size?: "sm" | "md" | "lg";
	className?: string;
}

const sizes = {
	sm: "size-8 text-xs",
	md: "size-10 text-sm",
	lg: "size-12 text-base",
};

export function IdentityAvatar({
	author,
	fallback,
	size = "md",
	className,
}: IdentityAvatarProps) {
	if (author?.avatar_url) {
		return (
			<img
				src={author.avatar_url}
				alt={displayAuthorName(author, fallback)}
				className={cn(
					"rounded-full border border-white/10 object-cover",
					sizes[size],
					className,
				)}
			/>
		);
	}

	return (
		<span
			className={cn(
				"inline-flex items-center justify-center rounded-full border border-white/10 bg-linear-to-br font-semibold",
				authorAvatarTone(author?.slack_user_id || fallback),
				sizes[size],
				className,
			)}
		>
			{authorInitials(author, fallback)}
		</span>
	);
}

import { cn } from "@/lib/utils";
import { type VariantProps, cva } from "class-variance-authority";
import type { ButtonHTMLAttributes } from "react";

export const buttonVariants = cva(
	"inline-flex items-center justify-center gap-2 rounded-(--radius-button) border text-sm font-medium transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/40 focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-bg-deep) disabled:pointer-events-none disabled:opacity-50",
	{
		variants: {
			variant: {
				default:
					"border-transparent bg-(--color-accent-strong) px-5 py-2.5 text-(--color-bg-deep) font-semibold shadow-[0_8px_24px_oklch(0.82_0.18_160/0.2)] hover:bg-(--color-accent)",
				secondary:
					"border-(--color-border-default) bg-(--color-bg-surface) px-5 py-2.5 text-(--color-text-primary) hover:border-(--color-border-accent) hover:bg-(--color-bg-elevated)",
				ghost:
					"border-transparent px-4 py-2 text-(--color-text-secondary) hover:text-(--color-text-primary) hover:bg-(--color-bg-surface)",
				destructive:
					"border-transparent bg-(--color-destructive) px-5 py-2.5 text-white hover:bg-(--color-destructive)/90",
			},
			size: {
				default: "h-11",
				sm: "h-9 px-4 text-xs",
				lg: "h-12 px-6 text-base",
				icon: "size-10",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	},
);

export interface ButtonProps
	extends ButtonHTMLAttributes<HTMLButtonElement>,
		VariantProps<typeof buttonVariants> {}

export function Button({ className, variant, size, ...props }: ButtonProps) {
	return (
		<button
			className={cn(buttonVariants({ variant, size, className }))}
			{...props}
		/>
	);
}

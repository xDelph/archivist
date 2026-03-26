import { cn } from "@/lib/utils";
import { type VariantProps, cva } from "class-variance-authority";
import { type ButtonHTMLAttributes, forwardRef } from "react";

export const buttonVariants = cva(
	"inline-flex items-center justify-center gap-2 rounded-(--radius-button) border text-sm font-medium transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/40 focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-bg-deep) active:translate-y-px disabled:pointer-events-none disabled:translate-y-0 disabled:opacity-50",
	{
		variants: {
			variant: {
				default:
					"border-transparent bg-(--color-accent-strong) px-5 py-2.5 font-semibold text-(--color-on-accent) shadow-[0_12px_28px_color-mix(in_srgb,var(--color-accent)_20%,transparent)] hover:bg-(--color-accent) hover:shadow-[0_14px_30px_color-mix(in_srgb,var(--color-accent)_24%,transparent)]",
				secondary:
					"border-(--color-border-default) bg-(--color-bg-surface) px-5 py-2.5 text-(--color-text-primary) hover:border-(--color-border-accent) hover:bg-(--color-bg-elevated) hover:text-(--color-text-primary)",
				ghost:
					"border-transparent px-4 py-2 text-(--color-text-secondary) hover:bg-(--color-bg-surface) hover:text-(--color-text-primary)",
				destructive:
					"border-transparent bg-(--color-destructive) px-5 py-2.5 text-(--color-on-destructive) hover:bg-(--color-destructive)/90",
			},
			size: {
				default: "h-11",
				sm: "h-11 px-4 text-[0.82rem] sm:h-9 sm:text-xs",
				lg: "h-12 px-6 text-base",
				icon: "size-11 sm:size-10",
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

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
	({ className, variant, size, ...props }, ref) => {
		return (
			<button
				ref={ref}
				className={cn(buttonVariants({ variant, size, className }))}
				{...props}
			/>
		);
	},
);

Button.displayName = "Button";

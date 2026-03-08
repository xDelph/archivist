import { cn } from "@/lib/utils";
import { type VariantProps, cva } from "class-variance-authority";
import type { ButtonHTMLAttributes } from "react";

export const buttonVariants = cva(
	"inline-flex items-center justify-center rounded-full border text-sm font-medium transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
	{
		variants: {
			variant: {
				default:
					"border-transparent bg-[var(--accent-strong)] px-5 py-3 text-slate-950 shadow-[0_12px_30px_rgba(125,244,199,0.24)] hover:bg-[var(--accent)]",
				secondary:
					"border-white/12 bg-white/6 px-5 py-3 text-white hover:border-white/20 hover:bg-white/10",
				ghost: "border-transparent px-4 py-2 text-white/72 hover:text-white",
			},
			size: {
				default: "h-11",
				sm: "h-9 px-4 text-xs uppercase tracking-[0.22em]",
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

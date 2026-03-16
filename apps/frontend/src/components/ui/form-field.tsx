import { cn } from "@/lib/utils";
import type {
	InputHTMLAttributes,
	ReactNode,
	SelectHTMLAttributes,
} from "react";
import { useId } from "react";

interface FormFieldProps {
	label: string;
	children: ReactNode;
	className?: string;
	labelClassName?: string;
	controlId: string;
}

function FormField({
	label,
	children,
	className,
	labelClassName,
	controlId,
}: FormFieldProps) {
	return (
		<div className={className}>
			<label
				htmlFor={controlId}
				className={cn(
					"text-copy-quiet mb-2 block text-[0.72rem] font-medium uppercase tracking-[0.2em]",
					labelClassName,
				)}
			>
				{label}
			</label>
			{children}
		</div>
	);
}

interface FormFieldShellProps {
	children: ReactNode;
	className?: string;
}

function FormFieldShell({ children, className }: FormFieldShellProps) {
	return (
		<div
			className={cn(
				"surface-input flex min-h-11 items-center gap-2.5 px-3 py-2.5 focus-within:border-(--color-border-accent) focus-within:bg-(--color-bg-surface) focus-within:shadow-[0_0_0_2px_rgba(32,203,116,0.12)] sm:min-h-10",
				className,
			)}
		>
			{children}
		</div>
	);
}

export interface InputFieldProps
	extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange" | "value"> {
	label: string;
	value: string;
	onValueChange: (value: string) => void;
	icon?: ReactNode;
	fieldClassName?: string;
	labelClassName?: string;
	shellClassName?: string;
	inputClassName?: string;
}

export function InputField({
	label,
	value,
	onValueChange,
	icon,
	fieldClassName,
	labelClassName,
	shellClassName,
	inputClassName,
	...props
}: InputFieldProps) {
	const controlId = props.id ?? useId();

	return (
		<FormField
			label={label}
			className={fieldClassName}
			labelClassName={labelClassName}
			controlId={controlId}
		>
			<FormFieldShell className={shellClassName}>
				{icon}
				<input
					{...props}
					id={controlId}
					value={value}
					onChange={(event) => onValueChange(event.target.value)}
					className={cn(
						"w-full bg-transparent text-[0.95rem] text-white outline-none placeholder:text-(--color-text-quiet)",
						inputClassName,
					)}
				/>
			</FormFieldShell>
		</FormField>
	);
}

export interface SelectFieldOption {
	value: string;
	label: string;
	key?: string;
}

export interface SelectFieldProps
	extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "onChange" | "value"> {
	label: string;
	value: string;
	onValueChange: (value: string) => void;
	options: readonly SelectFieldOption[];
	fieldClassName?: string;
	labelClassName?: string;
	shellClassName?: string;
	selectClassName?: string;
}

export function SelectField({
	label,
	value,
	onValueChange,
	options,
	fieldClassName,
	labelClassName,
	shellClassName,
	selectClassName,
	...props
}: SelectFieldProps) {
	const controlId = props.id ?? useId();

	return (
		<FormField
			label={label}
			className={fieldClassName}
			labelClassName={labelClassName}
			controlId={controlId}
		>
			<FormFieldShell className={shellClassName}>
				<select
					{...props}
					id={controlId}
					value={value}
					onChange={(event) => onValueChange(event.target.value)}
					className={cn(
						"w-full bg-transparent text-[0.95rem] text-white outline-none disabled:cursor-not-allowed disabled:text-(--color-text-quiet)",
						selectClassName,
					)}
				>
					{options.map((option, index) => (
						<option
							key={option.key ?? (option.value || `option-${index + 1}`)}
							value={option.value}
							className="bg-(--color-bg-input) text-white"
						>
							{option.label}
						</option>
					))}
				</select>
			</FormFieldShell>
		</FormField>
	);
}

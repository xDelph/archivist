import { Features } from "./sections/features";
import { Footer } from "./sections/footer";
import { Header } from "./sections/header";
import { Hero } from "./sections/hero";
import { HowItWorks } from "./sections/how-it-works";
import { OpenSource } from "./sections/open-source";
import { Problem } from "./sections/problem";

export function App() {
	return (
		<div className="flex min-h-dvh flex-col">
			<Header />
			<div className="flex-1 overflow-x-hidden">
				<main className="pb-10">
					<Hero />
					<Problem />
					<Features />
					<HowItWorks />
					<OpenSource />
				</main>
				<Footer />
			</div>
		</div>
	);
}

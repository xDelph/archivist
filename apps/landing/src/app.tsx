import { Features } from "./sections/features";
import { Footer } from "./sections/footer";
import { Header } from "./sections/header";
import { Hero } from "./sections/hero";
import { HowItWorks } from "./sections/how-it-works";
import { Offline } from "./sections/offline";
import { OpenSource } from "./sections/open-source";
import { Problem } from "./sections/problem";
import { ShowcaseAi } from "./sections/showcase-ai";
import { ShowcaseDetail } from "./sections/showcase-detail";
import { ShowcaseSearch } from "./sections/showcase-search";
import { SlackCommands } from "./sections/slack-commands";

export function App() {
	return (
		<div className="flex h-dvh flex-col overflow-hidden">
			<Header />
			<div className="flex-1 overflow-x-hidden overflow-y-auto">
				<main>
					<Hero />
					<Problem />
					<Features />
					<ShowcaseSearch />
					<ShowcaseAi />
					<ShowcaseDetail />
					<SlackCommands />
					<Offline />
					<HowItWorks />
					<OpenSource />
				</main>
				<Footer />
			</div>
		</div>
	);
}

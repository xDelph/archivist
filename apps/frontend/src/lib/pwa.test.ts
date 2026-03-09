import {
	registerAppServiceWorker,
	shouldRegisterServiceWorker,
} from "@/lib/pwa";

describe("pwa helpers", () => {
	it("registers only when production and service worker support are available", () => {
		expect(
			shouldRegisterServiceWorker({
				isProduction: true,
				serviceWorker: {
					register: async () => ({}) as ServiceWorkerRegistration,
				},
			}),
		).toBe(true);
		expect(
			shouldRegisterServiceWorker({
				isProduction: false,
				serviceWorker: {
					register: async () => ({}) as ServiceWorkerRegistration,
				},
			}),
		).toBe(false);
		expect(
			shouldRegisterServiceWorker({
				isProduction: true,
			}),
		).toBe(false);
	});

	it("returns false when registration fails", async () => {
		const registered = await registerAppServiceWorker(
			{
				register: async () => {
					throw new Error("offline");
				},
			},
			true,
		);

		expect(registered).toBe(false);
	});
});

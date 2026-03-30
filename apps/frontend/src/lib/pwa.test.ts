import {
	collectCurrentShellAssetUrls,
	disableAppServiceWorker,
	registerAppServiceWorker,
	shouldRegisterServiceWorker,
	warmOfflineShellAssets,
} from "@/lib/pwa";
import { describe, expect, it, vi } from "vitest";

describe("pwa helpers", () => {
	it("registers only when production and service worker support are available", () => {
		expect(
			shouldRegisterServiceWorker({
				serviceWorker: {
					register: async () => ({}) as ServiceWorkerRegistration,
				},
				isDevelopment: false,
			}),
		).toBe(true);
		expect(
			shouldRegisterServiceWorker({
				serviceWorker: {
					register: async () => ({}) as ServiceWorkerRegistration,
				},
				isDevelopment: true,
			}),
		).toBe(false);
		expect(shouldRegisterServiceWorker({})).toBe(false);
	});

	it("returns false when registration fails", async () => {
		const registered = await registerAppServiceWorker({
			register: async () => {
				throw new Error("offline");
			},
		});

		expect(registered).toBe(false);
	});

	it("unregisters existing service workers and clears offline caches", async () => {
		const unregister = vi.fn().mockResolvedValue(true);
		const getRegistrations = vi.fn().mockResolvedValue([{ unregister }]);
		const deleteCache = vi.fn().mockResolvedValue(true);

		const disabled = await disableAppServiceWorker(
			{ getRegistrations } as unknown as ServiceWorkerContainer,
			{ delete: deleteCache } as unknown as CacheStorage,
		);

		expect(disabled).toBe(true);
		expect(getRegistrations).toHaveBeenCalledTimes(1);
		expect(unregister).toHaveBeenCalledTimes(1);
		expect(deleteCache).toHaveBeenCalledWith("arkivist-shell-v3");
		expect(deleteCache).toHaveBeenCalledWith("arkivist-assets-v3");
		expect(deleteCache).toHaveBeenCalledWith("arkivist-shell-v2");
		expect(deleteCache).toHaveBeenCalledWith("arkivist-assets-v2");
	});

	it("collects current same-origin shell assets", () => {
		const documentRef = {
			querySelectorAll: () =>
				[
					{ href: "https://arkivist.test/assets/index.css" },
					{ href: "https://arkivist.test/assets/index.js" },
					{ src: "https://arkivist.test/assets/app.js" },
					{ href: "https://cdn.example.com/avatar.png" },
				] as Array<Partial<HTMLLinkElement | HTMLScriptElement>>,
		} as unknown as Document;
		const locationRef = {
			origin: "https://arkivist.test",
		} as Location;

		expect(collectCurrentShellAssetUrls(documentRef, locationRef)).toEqual([
			"/assets/index.css",
			"/assets/index.js",
			"/assets/app.js",
		]);
	});

	it("warms the shell and current assets after registration", async () => {
		const shellPut = vi.fn().mockResolvedValue(undefined);
		const assetPut = vi.fn().mockResolvedValue(undefined);
		const open = vi
			.fn()
			.mockResolvedValueOnce({ put: shellPut })
			.mockResolvedValueOnce({ put: assetPut });
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			type: "basic",
			clone: () => ({ ok: true, type: "basic" }),
		});
		const documentRef = {
			querySelectorAll: () =>
				[{ href: "https://arkivist.test/assets/index.css" }] as Array<
					Partial<HTMLLinkElement>
				>,
		} as unknown as Document;
		const locationRef = {
			origin: "https://arkivist.test",
			pathname: "/threads/C123:1",
		} as Location;

		const warmed = await warmOfflineShellAssets(
			{ open } as unknown as CacheStorage,
			documentRef,
			locationRef,
			fetchImpl as typeof fetch,
		);

		expect(warmed).toBe(true);
		expect(open).toHaveBeenNthCalledWith(1, "arkivist-shell-v3");
		expect(open).toHaveBeenNthCalledWith(2, "arkivist-assets-v3");
		expect(shellPut).toHaveBeenCalled();
		expect(assetPut).toHaveBeenCalled();
	});
});

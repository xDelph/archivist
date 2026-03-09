type ServiceWorkerRegistrar = Pick<ServiceWorkerContainer, "register">;

export interface ServiceWorkerEnvironment {
	isProduction: boolean;
	serviceWorker?: ServiceWorkerRegistrar;
}

export function shouldRegisterServiceWorker({
	isProduction,
	serviceWorker,
}: ServiceWorkerEnvironment) {
	return isProduction && typeof serviceWorker !== "undefined";
}

export async function registerAppServiceWorker(
	serviceWorker: ServiceWorkerRegistrar | undefined = globalThis.navigator
		?.serviceWorker,
	isProduction = import.meta.env.PROD,
) {
	if (
		!shouldRegisterServiceWorker({
			isProduction,
			serviceWorker,
		})
	) {
		return false;
	}

	try {
		await serviceWorker.register("/sw.js", { scope: "/" });
		return true;
	} catch {
		return false;
	}
}

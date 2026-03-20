import {
	dismissOfflineSaveNotification,
	getOfflineSaveNotificationSnapshot,
	notifyOfflineSaveError,
	notifyOfflineSavePending,
	notifyOfflineSaveSuccess,
	resetOfflineSaveNotificationForTests,
} from "@/lib/offline-save-notification";
import { afterEach, describe, expect, it, vi } from "vitest";

describe("offline save notification store", () => {
	afterEach(() => {
		resetOfflineSaveNotificationForTests();
		vi.useRealTimers();
	});

	it("shows pending state until a terminal result replaces it", () => {
		notifyOfflineSavePending("C123:1");

		expect(getOfflineSaveNotificationSnapshot()).toMatchObject({
			threadId: "C123:1",
			status: "pending",
			phase: "open",
		});
	});

	it("auto-dismisses the success state gracefully", () => {
		vi.useFakeTimers();
		notifyOfflineSaveSuccess("C123:1");

		expect(getOfflineSaveNotificationSnapshot()).toMatchObject({
			status: "success",
			phase: "open",
		});

		vi.advanceTimersByTime(4200);

		expect(getOfflineSaveNotificationSnapshot()).toMatchObject({
			status: "success",
			phase: "closing",
		});

		vi.advanceTimersByTime(280);

		expect(getOfflineSaveNotificationSnapshot()).toBeNull();
	});

	it("dismisses the matching thread notification", () => {
		notifyOfflineSaveError("C123:1");
		dismissOfflineSaveNotification("C123:1");

		expect(getOfflineSaveNotificationSnapshot()).toMatchObject({
			status: "error",
			phase: "closing",
		});
	});
});

import { useSyncExternalStore } from "react";

const NOTIFICATION_EXIT_MS = 280;
const NOTIFICATION_SUCCESS_MS = 4200;
const NOTIFICATION_ERROR_MS = 5200;

type OfflineSaveNotificationStatus = "pending" | "success" | "error";
type OfflineSaveNotificationPhase = "open" | "closing";

interface OfflineSaveNotificationState {
	threadId: string;
	status: OfflineSaveNotificationStatus;
	title: string;
	description: string;
	phase: OfflineSaveNotificationPhase;
}

type OfflineSaveNotificationListener = () => void;

let currentNotification: OfflineSaveNotificationState | null = null;
let dismissTimer: ReturnType<typeof setTimeout> | null = null;
let clearTimer: ReturnType<typeof setTimeout> | null = null;
const listeners = new Set<OfflineSaveNotificationListener>();

export function useOfflineSaveNotification() {
	return useSyncExternalStore(subscribe, getSnapshot);
}

export function getOfflineSaveNotificationSnapshot() {
	return currentNotification;
}

export function notifyOfflineSavePending(threadId: string) {
	showNotification({
		threadId,
		status: "pending",
		title: "Saving offline access",
		description: "Archivist is preparing saved content for offline reading.",
		phase: "open",
	});
}

export function notifyOfflineSaveSuccess(threadId: string) {
	showNotification({
		threadId,
		status: "success",
		title: "Available offline",
		description: "Saved content is ready for offline reading.",
		phase: "open",
	});
	scheduleDismiss(NOTIFICATION_SUCCESS_MS);
}

export function notifyOfflineSaveError(threadId: string) {
	showNotification({
		threadId,
		status: "error",
		title: "Offline copy incomplete",
		description: "Saved content may not be ready for offline reading yet.",
		phase: "open",
	});
	scheduleDismiss(NOTIFICATION_ERROR_MS);
}

export function dismissOfflineSaveNotification(threadId?: string) {
	if (!currentNotification) {
		return;
	}

	if (threadId && currentNotification.threadId !== threadId) {
		return;
	}

	beginDismiss();
}

export function resetOfflineSaveNotificationForTests() {
	clearScheduledTimers();
	currentNotification = null;
	emitChange();
}

function subscribe(listener: OfflineSaveNotificationListener) {
	listeners.add(listener);
	return () => listeners.delete(listener);
}

function getSnapshot() {
	return currentNotification;
}

function showNotification(notification: OfflineSaveNotificationState) {
	clearScheduledTimers();
	currentNotification = notification;
	emitChange();
}

function scheduleDismiss(delayMs: number) {
	clearDismissTimer();
	dismissTimer = setTimeout(() => beginDismiss(), delayMs);
}

function beginDismiss() {
	clearDismissTimer();
	if (!currentNotification || currentNotification.phase === "closing") {
		return;
	}

	currentNotification = {
		...currentNotification,
		phase: "closing",
	};
	emitChange();
	clearTimer = setTimeout(() => {
		currentNotification = null;
		clearTimer = null;
		emitChange();
	}, NOTIFICATION_EXIT_MS);
}

function clearScheduledTimers() {
	clearDismissTimer();
	if (clearTimer) {
		clearTimeout(clearTimer);
		clearTimer = null;
	}
}

function clearDismissTimer() {
	if (dismissTimer) {
		clearTimeout(dismissTimer);
		dismissTimer = null;
	}
}

function emitChange() {
	for (const listener of listeners) {
		listener();
	}
}

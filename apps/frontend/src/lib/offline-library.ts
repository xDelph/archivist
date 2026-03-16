import type { SavedItem, ThreadDetailResponse } from "@/lib/api";
import {
	cacheOfflineImage,
	cacheOfflineThreadAvatars,
} from "@/lib/offline-assets";

const OFFLINE_DB_NAME = "archivist-offline";
const OFFLINE_DB_VERSION = 1;
const SAVED_ITEMS_STORE = "saved-items";
const THREAD_DETAILS_STORE = "thread-details";

interface OfflineSavedItemRecord {
	threadId: string;
	item: SavedItem;
}

interface OfflineThreadDetailRecord {
	threadId: string;
	thread: ThreadDetailResponse;
}

export function supportsOfflineLibrary(
	indexedDb: IDBFactory | undefined = globalThis.indexedDB,
) {
	return typeof indexedDb !== "undefined";
}

export async function listOfflineSavedItems() {
	const database = await openOfflineDatabase();
	if (!database) {
		return [];
	}

	const records = await readAllRecords<OfflineSavedItemRecord>(
		database,
		SAVED_ITEMS_STORE,
	);
	return records.map((record) => record.item);
}

export async function listOfflineThreadDetailIds() {
	const database = await openOfflineDatabase();
	if (!database) {
		return [];
	}

	return readAllKeys(database, THREAD_DETAILS_STORE);
}

export async function getOfflineThreadDetail(threadId: string) {
	const database = await openOfflineDatabase();
	if (!database) {
		return null;
	}

	const record = await readRecord<OfflineThreadDetailRecord>(
		database,
		THREAD_DETAILS_STORE,
		threadId,
	);
	return record?.thread ?? null;
}

export async function replaceOfflineSavedItems(items: SavedItem[]) {
	const database = await openOfflineDatabase();
	if (!database) {
		return;
	}

	const allowedThreadIds = new Set(items.map((item) => item.thread_id));
	const existingThreadIds = await readAllKeys(database, THREAD_DETAILS_STORE);
	const transaction = database.transaction(
		[SAVED_ITEMS_STORE, THREAD_DETAILS_STORE],
		"readwrite",
	);
	const savedItemsStore = transaction.objectStore(SAVED_ITEMS_STORE);
	const threadDetailsStore = transaction.objectStore(THREAD_DETAILS_STORE);

	savedItemsStore.clear();
	for (const item of items) {
		savedItemsStore.put({
			threadId: item.thread_id,
			item,
		} satisfies OfflineSavedItemRecord);
	}

	for (const threadId of existingThreadIds) {
		if (!allowedThreadIds.has(threadId)) {
			threadDetailsStore.delete(threadId);
		}
	}

	await transactionDone(transaction);
}

export async function storeOfflineSavedThread(
	item: SavedItem,
	thread: ThreadDetailResponse,
) {
	const database = await openOfflineDatabase();
	if (!database) {
		return;
	}

	const transaction = database.transaction(
		[SAVED_ITEMS_STORE, THREAD_DETAILS_STORE],
		"readwrite",
	);
	transaction.objectStore(SAVED_ITEMS_STORE).put({
		threadId: item.thread_id,
		item,
	} satisfies OfflineSavedItemRecord);
	transaction.objectStore(THREAD_DETAILS_STORE).put({
		threadId: thread.id,
		thread,
	} satisfies OfflineThreadDetailRecord);
	await transactionDone(transaction);
	await cacheOfflineThreadAvatars(item, thread);
}

export async function storeOfflineThreadDetail(thread: ThreadDetailResponse) {
	const database = await openOfflineDatabase();
	if (!database) {
		return;
	}

	const transaction = database.transaction(THREAD_DETAILS_STORE, "readwrite");
	transaction.objectStore(THREAD_DETAILS_STORE).put({
		threadId: thread.id,
		thread,
	} satisfies OfflineThreadDetailRecord);
	await transactionDone(transaction);
	await Promise.all(
		thread.messages.map((message) =>
			cacheOfflineImage(message.author?.avatar_url),
		),
	);
}

export async function removeOfflineSavedThread(threadId: string) {
	const database = await openOfflineDatabase();
	if (!database) {
		return;
	}

	const transaction = database.transaction(
		[SAVED_ITEMS_STORE, THREAD_DETAILS_STORE],
		"readwrite",
	);
	transaction.objectStore(SAVED_ITEMS_STORE).delete(threadId);
	transaction.objectStore(THREAD_DETAILS_STORE).delete(threadId);
	await transactionDone(transaction);
}

async function openOfflineDatabase(
	indexedDb: IDBFactory | undefined = globalThis.indexedDB,
) {
	if (!supportsOfflineLibrary(indexedDb)) {
		return null;
	}

	const request = indexedDb.open(OFFLINE_DB_NAME, OFFLINE_DB_VERSION);
	request.onupgradeneeded = () => {
		const database = request.result;
		if (!database.objectStoreNames.contains(SAVED_ITEMS_STORE)) {
			database.createObjectStore(SAVED_ITEMS_STORE, {
				keyPath: "threadId",
			});
		}
		if (!database.objectStoreNames.contains(THREAD_DETAILS_STORE)) {
			database.createObjectStore(THREAD_DETAILS_STORE, {
				keyPath: "threadId",
			});
		}
	};

	return requestToPromise(request);
}

async function readAllRecords<T>(database: IDBDatabase, storeName: string) {
	const transaction = database.transaction(storeName, "readonly");
	const records = await requestToPromise<T[]>(
		transaction.objectStore(storeName).getAll(),
	);
	await transactionDone(transaction);
	return records;
}

async function readAllKeys(database: IDBDatabase, storeName: string) {
	const transaction = database.transaction(storeName, "readonly");
	const keys = await requestToPromise<string[]>(
		transaction.objectStore(storeName).getAllKeys(),
	);
	await transactionDone(transaction);
	return keys;
}

async function readRecord<T>(
	database: IDBDatabase,
	storeName: string,
	key: string,
) {
	const transaction = database.transaction(storeName, "readonly");
	const record = await requestToPromise<T | undefined>(
		transaction.objectStore(storeName).get(key),
	);
	await transactionDone(transaction);
	return record ?? null;
}

function requestToPromise<T>(request: IDBRequest<T>) {
	return new Promise<T>((resolve, reject) => {
		request.onsuccess = () => resolve(request.result);
		request.onerror = () =>
			reject(request.error ?? new Error("IndexedDB request failed"));
	});
}

function transactionDone(transaction: IDBTransaction) {
	return new Promise<void>((resolve, reject) => {
		transaction.oncomplete = () => resolve();
		transaction.onerror = () =>
			reject(transaction.error ?? new Error("IndexedDB transaction failed"));
		transaction.onabort = () =>
			reject(transaction.error ?? new Error("IndexedDB transaction aborted"));
	});
}

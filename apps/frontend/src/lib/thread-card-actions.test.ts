import {
	buildThreadCardActionKinds,
	buildThreadShareUrl,
} from "@/lib/thread-card-actions";

describe("thread card action helpers", () => {
	it("builds save and star add-actions for online admins", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: true,
				isAdmin: true,
				isSaved: false,
				isStarred: false,
			}),
		).toEqual({
			leading: [],
			trailing: ["save", "star"],
			menu: ["save", "star"],
			quick: ["save", "star"],
		});
	});

	it("builds remove-actions when a thread is already saved and starred", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: true,
				isAdmin: true,
				isSaved: true,
				isStarred: true,
			}),
		).toEqual({
			leading: ["unsave", "unstar"],
			trailing: [],
			menu: ["unsave", "unstar"],
			quick: ["unsave", "unstar"],
		});
	});

	it("hides all actions offline and admin-only actions for members", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: false,
				isAdmin: true,
				isSaved: true,
				isStarred: true,
			}),
		).toEqual({
			leading: [],
			trailing: [],
			menu: [],
			quick: [],
		});
		expect(
			buildThreadCardActionKinds({
				isOnline: true,
				isAdmin: false,
				isSaved: false,
				isStarred: false,
			}),
		).toEqual({
			leading: [],
			trailing: ["save"],
			menu: ["save"],
			quick: ["save"],
		});
	});

	it("builds an Arkivist thread URL for share actions", () => {
		expect(
			buildThreadShareUrl(
				"https://app.arkivist.test",
				"C123456:1742816123.004200",
			),
		).toBe("https://app.arkivist.test/threads/C123456%3A1742816123.004200");
	});
});

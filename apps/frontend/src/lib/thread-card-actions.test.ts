import { buildThreadCardActionKinds } from "@/lib/thread-card-actions";

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
		});
	});
});

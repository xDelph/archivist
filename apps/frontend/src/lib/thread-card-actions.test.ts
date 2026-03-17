import { buildThreadCardActionKinds } from "@/lib/thread-card-actions";

describe("thread card action helpers", () => {
	it("builds save and highlight add-actions for online admins", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: true,
				isAdmin: true,
				isSaved: false,
				isHighlighted: false,
			}),
		).toEqual({
			leading: [],
			trailing: ["save", "highlight"],
			menu: ["save", "highlight"],
		});
	});

	it("builds remove-actions when a thread is already saved and highlighted", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: true,
				isAdmin: true,
				isSaved: true,
				isHighlighted: true,
			}),
		).toEqual({
			leading: ["unsave", "unhighlight"],
			trailing: [],
			menu: ["unsave", "unhighlight"],
		});
	});

	it("hides all actions offline and admin-only actions for members", () => {
		expect(
			buildThreadCardActionKinds({
				isOnline: false,
				isAdmin: true,
				isSaved: true,
				isHighlighted: true,
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
				isHighlighted: false,
			}),
		).toEqual({
			leading: [],
			trailing: ["save"],
			menu: ["save"],
		});
	});
});

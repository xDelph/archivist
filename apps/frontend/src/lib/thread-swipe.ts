export const THREAD_SWIPE_ACTION_WIDTH = 92;
export const THREAD_SWIPE_ACTION_GAP = 6;
export const THREAD_SWIPE_ACTION_PADDING = 6;
const OPEN_THRESHOLD_RATIO = 0.52;

export function getThreadSwipeRailWidth(actionCount: number) {
	if (actionCount <= 0) {
		return 0;
	}

	return (
		actionCount * THREAD_SWIPE_ACTION_WIDTH +
		Math.max(0, actionCount - 1) * THREAD_SWIPE_ACTION_GAP +
		THREAD_SWIPE_ACTION_PADDING * 2
	);
}

export function clampSwipeOffset(
	offset: number,
	leadingWidth = 0,
	trailingWidth = 0,
) {
	return Math.min(leadingWidth, Math.max(-trailingWidth, offset));
}

export function resolveSwipeOffset(
	offset: number,
	leadingWidth = 0,
	trailingWidth = 0,
) {
	if (offset > 0) {
		return offset >= leadingWidth * OPEN_THRESHOLD_RATIO ? leadingWidth : 0;
	}

	if (offset < 0) {
		return Math.abs(offset) >= trailingWidth * OPEN_THRESHOLD_RATIO
			? -trailingWidth
			: 0;
	}

	return 0;
}

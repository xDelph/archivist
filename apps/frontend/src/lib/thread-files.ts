export function isImageFile(mimetype: string | null) {
	return mimetype?.startsWith("image/") ?? false;
}

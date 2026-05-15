const DEEP_LINK_PROTOCOL = "starlight:";
const PROFILE_DEEP_LINK_HOST = "profile";

export function parseProfileIdFromDeepLink(urlValue: string): string | null {
  try {
    const url = new URL(urlValue);
    if (url.protocol !== DEEP_LINK_PROTOCOL || url.hostname !== PROFILE_DEEP_LINK_HOST) {
      return null;
    }

    const segments = url.pathname.split("/").filter(Boolean);
    if (segments.length !== 1) {
      return null;
    }

    const profileId = decodeURIComponent(segments[0]).trim();
    return profileId.length > 0 ? profileId : null;
  } catch {
    return null;
  }
}

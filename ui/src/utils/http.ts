export const METHOD_COLORS: Record<string, string> = {
  GET: "var(--method-get)",
  POST: "var(--method-post)",
  PUT: "var(--method-put)",
  PATCH: "var(--method-patch)",
  DELETE: "var(--method-delete)",
  HEAD: "var(--text-secondary)",
  OPTIONS: "var(--text-secondary)",
};

/**
 * Detect body content type from a Content-Type header string.
 * Returns "json", "xml", "form-urlencoded", "text", or the provided fallback.
 */
export function detectContentType(
  contentTypeHeader: string,
  fallback: "json" | "xml" | "text" | "form-urlencoded" | "none" = "json",
): "json" | "xml" | "text" | "form-urlencoded" | "none" {
  if (contentTypeHeader.includes("json")) return "json";
  if (contentTypeHeader.includes("xml")) return "xml";
  if (contentTypeHeader.includes("form-urlencoded")) return "form-urlencoded";
  if (contentTypeHeader.includes("text")) return "text";
  return fallback;
}

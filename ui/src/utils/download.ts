/**
 * Trigger a file save via the File System Access API (showSaveFilePicker)
 * when available, falling back to an anchor-download approach.
 */
export async function triggerDownload(content: string | object, filename: string, mimeType: string): Promise<void> {
  const text = typeof content === "string" ? content : JSON.stringify(content, null, 2);
  const blob = new Blob([text], { type: mimeType });

  // Try native Save As dialog (modern browsers)
  if ("showSaveFilePicker" in window) {
    try {
      const ext = filename.includes(".") ? filename.slice(filename.lastIndexOf(".")) : "";
      const handle = await (window as any).showSaveFilePicker({
        suggestedName: filename,
        types: ext
          ? [{ description: "Export file", accept: { [mimeType]: [ext] } }]
          : undefined,
      });
      const writable = await handle.createWritable();
      await writable.write(blob);
      await writable.close();
      return;
    } catch (err: any) {
      // User cancelled the dialog
      if (err?.name === "AbortError") throw err;
      // API failed — fall through to anchor approach
    }
  }

  // Fallback: anchor download
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Copy text to the system clipboard.
 *
 * Prefer the async Clipboard API. On plain HTTP (typical for camera IPs)
 * `navigator.clipboard` is undefined, so fall back to a hidden textarea +
 * `document.execCommand('copy')`.
 */
export async function copyTextToClipboard(text: string): Promise<boolean> {
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      // Permission denied or other Clipboard API failure — try the legacy path.
    }
  }
  return copyViaExecCommand(text);
}

function copyViaExecCommand(text: string): boolean {
  if (typeof document === 'undefined') {
    return false;
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', '');
  textarea.style.position = 'fixed';
  textarea.style.left = '-9999px';
  textarea.style.top = '0';
  document.body.appendChild(textarea);

  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, text.length);

  try {
    // Legacy fallback for plain-HTTP camera UIs where Clipboard API is unavailable.
    return document.execCommand('copy'); // NOSONAR -- intentional HTTP clipboard fallback
  } catch {
    return false;
  } finally {
    textarea.remove();
  }
}

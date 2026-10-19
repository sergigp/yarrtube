const LANGUAGE_NAMES = new Intl.DisplayNames(['en'], { type: 'language' })

/** The English name of a language code (e.g. "English" for `en`), or the code itself when unknown. */
export function languageName(code: string): string {
  try {
    return LANGUAGE_NAMES.of(code) ?? code
  } catch {
    // A malformed code is rejected with a RangeError: show it as-is.
    return code
  }
}

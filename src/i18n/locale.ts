import { invoke } from "@tauri-apps/api/core";

export type Locale = "zh-Hans" | "zh-Hant" | "en-US" | "en-AU" | "ja";

/** Same mapping as the Rust side (src-tauri/src/i18n/mod.rs), for previews outside Tauri. */
export function fromTag(tag: string): Locale | null {
  const [lang, ...rest] = tag.trim().replace(/_/g, "-").toLowerCase().split("-");
  const has = (x: string) => rest.includes(x);
  if (lang === "zh") return has("hant") || (!has("hans") && (has("tw") || has("hk") || has("mo"))) ? "zh-Hant" : "zh-Hans";
  if (lang === "yue") return "zh-Hant";
  if (lang === "en") return ["au", "nz", "gb", "ie"].some(has) ? "en-AU" : "en-US";
  if (lang === "ja") return "ja";
  return null;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * The language of this run. In the app it is what Rust settled at launch (the system's),
 * so every window and the menus agree; in a plain browser preview `?lang=` or the browser's.
 */
export const locale: Locale = inTauri
  ? await invoke<Locale>("get_locale")
  : (fromTag(new URLSearchParams(location.search).get("lang") ?? "") ?? navigator.languages.map(fromTag).find((l) => l !== null) ?? "en-US");

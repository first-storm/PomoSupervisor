import commonEnAU from "./common/en-AU";
import commonEnUS from "./common/en-US";
import commonJa from "./common/ja";
import commonZhHans from "./common/zh-Hans";
import commonZhHant from "./common/zh-Hant";
import { type Locale, locale } from "./locale";
import popoverEnAU from "./popover/en-AU";
import popoverEnUS from "./popover/en-US";
import popoverJa from "./popover/ja";
import popoverZhHans from "./popover/zh-Hans";
import popoverZhHant from "./popover/zh-Hant";
import reviewEnAU from "./review/en-AU";
import reviewEnUS from "./review/en-US";
import reviewJa from "./review/ja";
import reviewZhHans from "./review/zh-Hans";
import reviewZhHant from "./review/zh-Hant";
import settingsEnAU from "./settings/en-AU";
import settingsEnUS from "./settings/en-US";
import settingsJa from "./settings/ja";
import settingsZhHans from "./settings/zh-Hans";
import settingsZhHant from "./settings/zh-Hant";

export { type Locale, locale };

const dicts = {
  "zh-Hans": { common: commonZhHans, popover: popoverZhHans, settings: settingsZhHans, review: reviewZhHans },
  "zh-Hant": { common: commonZhHant, popover: popoverZhHant, settings: settingsZhHant, review: reviewZhHant },
  "en-US": { common: commonEnUS, popover: popoverEnUS, settings: settingsEnUS, review: reviewEnUS },
  "en-AU": { common: commonEnAU, popover: popoverEnAU, settings: settingsEnAU, review: reviewEnAU },
  ja: { common: commonJa, popover: popoverJa, settings: settingsJa, review: reviewJa },
} satisfies Record<Locale, unknown>;

/** The strings of this run's language. The language is fixed for the life of the webview. */
export const t = dicts[locale];

/** The tag to hand Intl / toLocale*String. */
export const intlTag = ({ "zh-Hans": "zh-CN", "zh-Hant": "zh-TW", "en-US": "en-US", "en-AU": "en-AU", ja: "ja-JP" } as const)[locale];

document.documentElement.lang = locale;

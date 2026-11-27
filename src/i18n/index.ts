import { locale, type Locale } from "./locale";
import commonZhHans from "./common/zh-Hans";
import commonZhHant from "./common/zh-Hant";
import commonEnUS from "./common/en-US";
import commonEnAU from "./common/en-AU";
import commonJa from "./common/ja";
import popoverZhHans from "./popover/zh-Hans";
import popoverZhHant from "./popover/zh-Hant";
import popoverEnUS from "./popover/en-US";
import popoverEnAU from "./popover/en-AU";
import popoverJa from "./popover/ja";
import settingsZhHans from "./settings/zh-Hans";
import settingsZhHant from "./settings/zh-Hant";
import settingsEnUS from "./settings/en-US";
import settingsEnAU from "./settings/en-AU";
import settingsJa from "./settings/ja";
import reviewZhHans from "./review/zh-Hans";
import reviewZhHant from "./review/zh-Hant";
import reviewEnUS from "./review/en-US";
import reviewEnAU from "./review/en-AU";
import reviewJa from "./review/ja";

export { locale, type Locale };

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

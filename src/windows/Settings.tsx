import { useEffect, useRef, useState } from "react";
import { api, errText, onWindowClosing, onWindowShown, type ModelStatus, type ProviderConfig, type ProviderKind, type Rules, type Settings, type Slot } from "../lib/api";
import { LockIcon } from "../components/ui";
import { t } from "../i18n";

type PaneId = "general" | "chain" | "lists" | "models" | "privacy";
const PANES: [PaneId, string][] = [
  ["general", t.settings.panes.general],
  ["chain", t.settings.panes.chain],
  ["lists", t.settings.panes.lists],
  ["models", t.settings.panes.models],
  ["privacy", t.settings.panes.privacy],
];

const KIND_LABEL: Record<ProviderKind, string> = t.settings.kind;
const URL_HINT: Record<ProviderKind, string> = t.settings.urlHint;
const MODEL_HINT: Record<ProviderKind, Record<Slot, string>> = t.settings.modelHint;

/** One-click fills, so setting a model up is "pick one, paste the key". */
const PRESETS: { name: string; slots: Slot[]; cfg: (slot: Slot) => ProviderConfig }[] = [
  { name: "Claude Haiku", slots: ["text", "vision"], cfg: () => ({ kind: "anthropic", base_url: "", model: "claude-haiku-5-5" }) },
  { name: "DeepSeek", slots: ["text"], cfg: () => ({ kind: "openai", base_url: "https://api.deepseek.com/v1", model: "deepseek-chat" }) },
  { name: "OpenAI", slots: ["text", "vision"], cfg: () => ({ kind: "openai", base_url: "", model: "gpt-4o-mini" }) },
  { name: t.settings.presetOllama, slots: ["text", "vision"], cfg: (slot) => ({ kind: "ollama", base_url: "", model: slot === "text" ? "qwen2.5:7b" : "qwen2.5vl:7b" }) },
];
const sameCfg = (a: ProviderConfig, b: ProviderConfig) => a.kind === b.kind && a.base_url === b.base_url && a.model === b.model;

type Update = (patch: Partial<Settings>) => void;

/** A select over presets that keeps the saved value even when it isn't one of them. */
function Choice({ label, value, options, fmt, onChange }: { label: string; value: number; options: number[]; fmt: (n: number) => string; onChange: (n: number) => void }) {
  const all = options.includes(value) ? options : [...options, value].sort((a, b) => a - b);
  return (
    <select className="select" aria-label={label} value={value} onChange={(e) => onChange(Number(e.target.value))}>
      {all.map((n) => (
        <option key={n} value={n}>
          {fmt(n)}
        </option>
      ))}
    </select>
  );
}

function NumberIn({ label, value, min, max, onChange }: { label: string; value: number; min: number; max: number; onChange: (n: number) => void }) {
  const [text, setText] = useState(String(value));
  useEffect(() => setText(String(value)), [value]);
  const commit = () => {
    const n = Math.round(Number(text));
    if (Number.isFinite(n) && n >= min && n <= max) onChange(n);
    else setText(String(value));
  };
  return (
    <input
      className="input num-in num"
      aria-label={label}
      inputMode="numeric"
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && commit()}
    />
  );
}

function Row({ step, name, tag, desc, children }: { step?: number; name: string; tag?: "local" | "model"; desc: React.ReactNode; children?: React.ReactNode }) {
  return (
    <div className="srow">
      {step != null && (
        <span className="stepno" aria-hidden="true">
          {step}
        </span>
      )}
      <div className="what">
        <div className="name">
          <span>{name}</span>
          {tag === "local" && <span className="badge sm">{t.settings.tagLocal}</span>}
          {tag === "model" && <span className="badge sm model">{t.settings.tagModel}</span>}
        </div>
        <div className="desc">{desc}</div>
      </div>
      {children}
    </div>
  );
}

const secs = t.settings.everySecs;
const span = t.settings.span;

function GeneralPane({ s, update }: { s: Settings; update: Update }) {
  return (
    <>
      <div>
        <h1>{t.settings.general.title}</h1>
        <div className="lede">{t.settings.general.lede}</div>
      </div>
      <div className="rows">
        <Row name={t.settings.general.focus} desc={t.settings.general.focusDesc}>
          <NumberIn label={t.settings.general.focus} value={s.focus_min} min={1} max={180} onChange={(n) => update({ focus_min: n })} />
          <span className="unit">{t.settings.general.minUnit}</span>
        </Row>
        <Row name={t.settings.general.shortBreak} desc={t.settings.general.shortBreakDesc}>
          <NumberIn label={t.settings.general.shortBreak} value={s.short_break_min} min={1} max={60} onChange={(n) => update({ short_break_min: n })} />
          <span className="unit">{t.settings.general.minUnit}</span>
        </Row>
        <Row name={t.settings.general.longBreak} desc={t.settings.general.longBreakDesc(s.long_break_every)}>
          <NumberIn label={t.settings.general.longBreak} value={s.long_break_min} min={1} max={120} onChange={(n) => update({ long_break_min: n })} />
          <span className="unit">{t.settings.general.minUnit}</span>
        </Row>
        <Row name={t.settings.general.longBreakEvery} desc={t.settings.general.longBreakEveryDesc}>
          <NumberIn label={t.settings.general.longBreakEvery} value={s.long_break_every} min={1} max={12} onChange={(n) => update({ long_break_every: n })} />
          <span className="unit">{t.settings.general.longBreakEveryUnit(s.long_break_every)}</span>
        </Row>
        <Row name={t.settings.general.sound} desc={t.settings.general.soundDesc}>
          <input type="checkbox" className="switch" aria-label={t.settings.general.sound} checked={s.sound} onChange={(e) => update({ sound: e.target.checked })} />
        </Row>
        <Row name={t.settings.general.autoResume} desc={t.settings.general.autoResumeDesc}>
          <input type="checkbox" className="switch" aria-label={t.settings.general.autoResume} checked={s.auto_resume} onChange={(e) => update({ auto_resume: e.target.checked })} />
        </Row>
      </div>
    </>
  );
}

function ChainPane({ s, update, go }: { s: Settings; update: Update; go: (p: PaneId) => void }) {
  const r = s.rules;
  const blocked = r.block_apps.length + r.block_domains.length + r.block_keywords.length;
  const allowed = r.allow_apps.length + r.allow_domains.length + r.allow_keywords.length;
  const modelName = (m: ProviderConfig | null) => (m?.model ? m.model : t.settings.chain.notConfigured);
  return (
    <>
      <div>
        <h1>{t.settings.chain.title}</h1>
        <div className="lede">{t.settings.chain.lede}</div>
      </div>
      <div className="rows">
        <Row step={1} name={t.settings.chain.sample} tag="local" desc={t.settings.chain.sampleDesc}>
          <Choice label={t.settings.chain.sampleLabel} value={s.sample_secs} options={[3, 4, 5, 10]} fmt={secs} onChange={(n) => update({ sample_secs: n })} />
        </Row>
        <Row step={2} name={t.settings.chain.idle} tag="local" desc={t.settings.chain.idleDesc}>
          <Choice label={t.settings.chain.idleLabel} value={s.idle_secs} options={[60, 120, 300, 600]} fmt={span} onChange={(n) => update({ idle_secs: n })} />
        </Row>
        <Row step={3} name={t.settings.chain.rules} tag="local" desc={t.settings.chain.rulesDesc(blocked, allowed)}>
          <button className="btn small" style={{ fontSize: 13 }} onClick={() => go("lists")}>
            {t.settings.chain.editLists}
          </button>
        </Row>
        <Row step={4} name={t.settings.chain.text} tag="model" desc={t.settings.chain.textDesc}>
          <button className="select" onClick={() => go("models")}>
            {modelName(s.text_model)}
          </button>
          <input type="checkbox" className="switch" aria-label={t.settings.chain.textEnable} checked={s.text_enabled} onChange={(e) => update({ text_enabled: e.target.checked })} />
        </Row>
        <Row step={5} name={t.settings.chain.vision} tag="model" desc={t.settings.chain.visionDesc}>
          <button className="select" onClick={() => go("models")}>
            {modelName(s.vision_model)}
          </button>
          <input type="checkbox" className="switch" aria-label={t.settings.chain.visionEnable} checked={s.vision_enabled} onChange={(e) => update({ vision_enabled: e.target.checked })} />
        </Row>
        <Row step={6} name={t.settings.chain.remind} tag="local" desc={t.settings.chain.remindDesc}>
          <Choice label={t.settings.chain.streakLabel} value={s.streak} options={[2, 3, 5]} fmt={t.settings.chain.streak} onChange={(n) => update({ streak: n })} />
        </Row>
      </div>
      <div className="foot-note">
        <LockIcon />
        <span>{t.settings.chain.footNote}</span>
      </div>
    </>
  );
}

function ListBox({ label, value, onChange }: { label: string; value: string[]; onChange: (v: string[]) => void }) {
  const [text, setText] = useState(value.join("\n"));
  useEffect(() => setText(value.join("\n")), [value]);
  return (
    <label className="listbox">
      <span className="head">
        <span className="label">{label}</span>
        <span className="cap num">{t.settings.lists.count(value.length)}</span>
      </span>
      <textarea
        rows={5}
        spellCheck={false}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => onChange(text.split("\n").map((x) => x.trim()).filter(Boolean))}
      />
    </label>
  );
}

function ListsPane({ s, update }: { s: Settings; update: Update }) {
  const set = (k: keyof Rules) => (v: string[]) => update({ rules: { ...s.rules, [k]: v } });
  return (
    <>
      <div>
        <h1>{t.settings.lists.title}</h1>
        <div className="lede">{t.settings.lists.lede}</div>
      </div>
      <div className="lists">
        <ListBox label={t.settings.lists.blockApps} value={s.rules.block_apps} onChange={set("block_apps")} />
        <ListBox label={t.settings.lists.allowApps} value={s.rules.allow_apps} onChange={set("allow_apps")} />
        <ListBox label={t.settings.lists.blockDomains} value={s.rules.block_domains} onChange={set("block_domains")} />
        <ListBox label={t.settings.lists.allowDomains} value={s.rules.allow_domains} onChange={set("allow_domains")} />
        <ListBox label={t.settings.lists.blockKeywords} value={s.rules.block_keywords} onChange={set("block_keywords")} />
        <ListBox label={t.settings.lists.allowKeywords} value={s.rules.allow_keywords} onChange={set("allow_keywords")} />
      </div>
      <div className="hint">{t.settings.lists.hint}</div>
    </>
  );
}

function TextIn({ label, value, placeholder, onChange }: { label: string; value: string; placeholder: string; onChange: (v: string) => void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <input
      className="input"
      aria-label={label}
      spellCheck={false}
      value={text}
      placeholder={placeholder}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => text !== value && onChange(text.trim())}
      onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
    />
  );
}

function Provider({ slot, title, cfg, onChange, status, onKeySaved }: {
  slot: Slot; title: string; cfg: ProviderConfig | null; onChange: (c: ProviderConfig | null) => void; status: ModelStatus | null; onKeySaved: () => void;
}) {
  const [key, setKey] = useState("");
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const hasKey = slot === "text" ? status?.text_key : status?.vision_key;
  const ready = slot === "text" ? status?.text_ready : status?.vision_ready;
  const buildErr = slot === "text" ? status?.text_error : status?.vision_error;

  return (
    <div className="provider">
      <div className="top">
        <div className="name">
          {title}
          {cfg && <span className={ready ? "status ok" : "status no"}>{ready ? t.settings.provider.ready : t.settings.provider.notReady}</span>}
        </div>
        <input
          type="checkbox"
          className="switch"
          aria-label={t.settings.provider.configure[slot]}
          checked={!!cfg}
          onChange={(e) => onChange(e.target.checked ? PRESETS[0].cfg(slot) : null)}
        />
      </div>
      {cfg && (
        <div className="pgrid">
          <label>{t.settings.provider.quickFill}</label>
          <div className="chips" role="group" aria-label={t.settings.provider.presets}>
            {PRESETS.filter((p) => p.slots.includes(slot)).map((p) => (
              <button key={p.name} className="chip" aria-pressed={sameCfg(cfg, p.cfg(slot))} onClick={() => onChange(p.cfg(slot))}>
                {p.name}
              </button>
            ))}
          </div>
          <label htmlFor={`${slot}-kind`}>{t.settings.provider.kind}</label>
          <select id={`${slot}-kind`} className="select" value={cfg.kind} onChange={(e) => onChange({ ...cfg, kind: e.target.value as ProviderKind })}>
            {(Object.keys(KIND_LABEL) as ProviderKind[]).map((k) => (
              <option key={k} value={k}>
                {KIND_LABEL[k]}
              </option>
            ))}
          </select>
          <label>Base URL</label>
          <TextIn label="Base URL" value={cfg.base_url} placeholder={URL_HINT[cfg.kind]} onChange={(v) => onChange({ ...cfg, base_url: v })} />
          <label>{t.settings.provider.model}</label>
          <TextIn label={t.settings.provider.model} value={cfg.model} placeholder={MODEL_HINT[cfg.kind][slot]} onChange={(v) => onChange({ ...cfg, model: v })} />
          {cfg.kind !== "ollama" && (
            <>
              <label htmlFor={`${slot}-key`}>API Key</label>
              <div className="inline">
                <input
                  id={`${slot}-key`}
                  className="input"
                  style={{ flex: 1 }}
                  type="password"
                  value={key}
                  placeholder={hasKey ? t.settings.provider.keyStored : "sk-..."}
                  onChange={(e) => setKey(e.target.value)}
                />
                <button
                  className="btn small"
                  disabled={!key}
                  onClick={async () => {
                    try {
                      await api.setApiKey(slot, key);
                      setKey("");
                      onKeySaved();
                      setMsg({ ok: true, text: t.settings.provider.keySaved });
                    } catch (e) {
                      setMsg({ ok: false, text: errText(e) });
                    }
                  }}
                >
                  {t.settings.provider.save}
                </button>
              </div>
            </>
          )}
          <span />
          <div className="inline">
            <button
              className="btn small"
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                setMsg(null);
                try {
                  setMsg({ ok: true, text: await api.testProvider(slot) });
                } catch (e) {
                  setMsg({ ok: false, text: errText(e) });
                } finally {
                  setBusy(false);
                }
              }}
            >
              {busy ? t.settings.provider.testing : t.settings.provider.test}
            </button>
          </div>
          {(msg || buildErr) && (
            <>
              <span />
              <div className={msg ? (msg.ok ? "msg good" : "msg bad") : "msg bad"}>{msg ? msg.text : buildErr}</div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

function ModelsPane({ s, update }: { s: Settings; update: Update }) {
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const refresh = () => api.modelStatus().then(setStatus);
  useEffect(() => {
    refresh();
  }, [s.text_model, s.vision_model]);
  return (
    <>
      <div>
        <h1>{t.settings.models.title}</h1>
        <div className="lede">{t.settings.models.lede}</div>
      </div>
      <div className="rows">
        <Provider slot="text" title={t.settings.models.text} cfg={s.text_model} status={status} onKeySaved={refresh} onChange={(c) => update({ text_model: c })} />
        <Provider slot="vision" title={t.settings.models.vision} cfg={s.vision_model} status={status} onKeySaved={refresh} onChange={(c) => update({ vision_model: c })} />
      </div>
      <details className="why">
        <summary>{t.settings.models.advanced}</summary>
      <div className="rows" style={{ marginTop: 10 }}>
        <Row name={t.settings.models.confidence} desc={t.settings.models.confidenceDesc}>
          <input
            type="range"
            aria-label={t.settings.models.confidence}
            min={0.5}
            max={0.95}
            step={0.05}
            value={s.confidence}
            onChange={(e) => update({ confidence: Number(e.target.value) })}
            style={{ accentColor: "var(--tomato)" }}
          />
          <span className="unit num" style={{ width: 32, textAlign: "right" }}>
            {Math.round(s.confidence * 100)}%
          </span>
        </Row>
        <Row name={t.settings.models.dedup} desc={t.settings.models.dedupDesc}>
          <NumberIn label={t.settings.models.dedupLabel} value={s.phash_threshold} min={0} max={32} onChange={(n) => update({ phash_threshold: n })} />
        </Row>
      </div>
      </details>
    </>
  );
}

function PrivacyPane({ s, update }: { s: Settings; update: Update }) {
  const [granted, setGranted] = useState<boolean | null>(null);
  const [axGranted, setAxGranted] = useState<boolean | null>(null);
  const check = () =>
    api.checkPermissions().then((p) => {
      setGranted(p.screen_recording);
      setAxGranted(p.accessibility);
    });
  useEffect(() => {
    check();
    const t = setInterval(check, 4000);
    return () => clearInterval(t);
  }, []);
  return (
    <>
      <div>
        <h1>{t.settings.privacy.title}</h1>
        <div className="lede">{t.settings.privacy.lede}</div>
      </div>
      <div className="rows">
        <Row name={t.settings.privacy.screen} desc={t.settings.privacy.screenDesc}>
          <span className={granted ? "status ok" : "status no"}>{granted == null ? "" : granted ? t.settings.privacy.granted : t.settings.privacy.notGranted}</span>
          {!granted && (
            <button
              className="btn small"
              onClick={async () => {
                await api.requestScreenRecording();
                await api.openPrivacy("screen");
              }}
            >
              {t.settings.privacy.grant}
            </button>
          )}
        </Row>
        <Row
          name={t.settings.privacy.ax}
          desc={t.settings.privacy.axDesc}
        >
          {s.ax_enabled && <span className={axGranted ? "status ok" : "status no"}>{axGranted == null ? "" : axGranted ? t.settings.privacy.granted : t.settings.privacy.notGranted}</span>}
          {s.ax_enabled && !axGranted && (
            <button
              className="btn small"
              onClick={async () => {
                await api.requestAccessibility();
                await api.openPrivacy("accessibility");
              }}
            >
              {t.settings.privacy.grant}
            </button>
          )}
          <input type="checkbox" className="switch" aria-label={t.settings.privacy.axToggle} checked={s.ax_enabled} onChange={(e) => update({ ax_enabled: e.target.checked })} />
        </Row>
        <Row name={t.settings.privacy.automation} desc={t.settings.privacy.automationDesc}>
          <button className="btn small" onClick={() => api.openPrivacy("automation")}>
            {t.settings.privacy.openSettings}
          </button>
        </Row>
        <Row name={t.settings.privacy.sendText} desc={t.settings.privacy.sendTextDesc} />
        <Row name={t.settings.privacy.sendVision} desc={t.settings.privacy.sendVisionDesc} />
        <Row name={t.settings.privacy.localData} desc={t.settings.privacy.localDataDesc} />
      </div>
      <div className="foot-note">
        <LockIcon />
        <span>{t.settings.privacy.footNote}</span>
      </div>
    </>
  );
}

export default function SettingsWindow() {
  const [pane, setPane] = useState<PaneId>("general");
  const [s, setS] = useState<Settings | null>(null);
  const [err, setErr] = useState("");
  const latest = useRef<Settings | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const dirty = useRef(false);

  const save = async () => {
    window.clearTimeout(timer.current);
    const next = latest.current;
    if (!dirty.current || !next) return;
    dirty.current = false;
    try {
      const saved = await api.saveSettings(next);
      setErr("");
      if (latest.current === next) {
        latest.current = saved;
        setS(saved);
      }
    } catch (e) {
      setErr(errText(e));
    }
  };

  useEffect(() => {
    const load = () =>
      api.getSettings().then((x) => {
        latest.current = x;
        dirty.current = false;
        setS(x);
      });
    // Another window may have asked for a particular pane (see openSettings).
    const jump = () => {
      const want = localStorage.getItem("settings.pane");
      localStorage.removeItem("settings.pane");
      if (want && PANES.some(([id]) => id === want)) setPane(want as PaneId);
    };
    load();
    jump();
    const subs = [
      onWindowShown(() => {
        load();
        jump();
      }),
      // Closing: commit the field being edited (its blur saves it), flush, then let the window go.
      onWindowClosing(async () => {
        (document.activeElement as HTMLElement | null)?.blur();
        await save();
        api.closeWindow();
      }),
    ];
    return () => subs.forEach((p) => p.then((un) => un()));
  }, []);

  // No Save button: every change is saved a moment after it's made.
  const update: Update = (patch) => {
    if (!latest.current) return;
    const next = { ...latest.current, ...patch };
    latest.current = next;
    dirty.current = true;
    setS(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(save, 300);
  };

  return (
    <div className="settings">
      <nav className="sidebar" aria-label={t.settings.sidebar} data-tauri-drag-region>
        {PANES.map(([id, name]) => (
          <button key={id} aria-current={pane === id ? "page" : undefined} onClick={() => setPane(id)}>
            {name}
          </button>
        ))}
      </nav>
      <main className="pane" data-tauri-drag-region>
        {err && <div className="msg bad">{t.settings.saveFailed(err)}</div>}
        {s && pane === "general" && <GeneralPane s={s} update={update} />}
        {s && pane === "chain" && <ChainPane s={s} update={update} go={setPane} />}
        {s && pane === "lists" && <ListsPane s={s} update={update} />}
        {s && pane === "models" && <ModelsPane s={s} update={update} />}
        {s && pane === "privacy" && <PrivacyPane s={s} update={update} />}
      </main>
    </div>
  );
}

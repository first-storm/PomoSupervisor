import { useEffect, useRef, useState } from "react";
import { api, errText, onWindowClosing, onWindowShown, type ModelStatus, type ProviderConfig, type ProviderKind, type Rules, type Settings, type Slot } from "../lib/api";
import { LockIcon } from "../components/ui";

type PaneId = "general" | "chain" | "lists" | "models" | "privacy";
const PANES: [PaneId, string][] = [
  ["general", "通用"],
  ["chain", "判断流程"],
  ["lists", "黑白名单"],
  ["models", "模型与密钥"],
  ["privacy", "隐私"],
];

const KIND_LABEL: Record<ProviderKind, string> = {
  openai: "OpenAI 兼容",
  anthropic: "Anthropic",
  ollama: "Ollama（本地）",
  decision: "Decision 模型（System One 兼容）",
  openai_decisions: "Decision 模型（OpenAI）",
};
const URL_HINT: Record<ProviderKind, string> = {
  openai: "留空为 OpenAI 官方；其他需带 /v1",
  anthropic: "留空为官方地址",
  ollama: "留空为 http://localhost:11434",
  decision: "完整 endpoint；留空为 TypeSafe，如 https://openrouter.ai/api/alpha/decisions",
  openai_decisions: "完整 endpoint；留空为 https://api.openai.com/v1/decisions",
};
const MODEL_HINT: Record<ProviderKind, Record<Slot, string>> = {
  openai: { text: "如 deepseek-chat / gpt-4o-mini", vision: "需支持图片输入的模型" },
  anthropic: { text: "如 claude-haiku-5-5", vision: "如 claude-haiku-5-5" },
  ollama: { text: "如 qwen2.5:7b", vision: "如 qwen2.5vl:7b" },
  decision: { text: "如 jev-latest / typesafe/jev-1.14", vision: "需支持图片，如 jev-1.14" },
  openai_decisions: { text: "如 gpt-6-luna", vision: "如 gpt-6-luna" },
};

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
          {tag === "local" && <span className="badge sm">本地</span>}
          {tag === "model" && <span className="badge sm model">调用模型</span>}
        </div>
        <div className="desc">{desc}</div>
      </div>
      {children}
    </div>
  );
}

const secs = (n: number) => `每 ${n} 秒`;
const span = (n: number) => (n % 60 === 0 ? `${n / 60} 分钟` : `${n} 秒`);

function GeneralPane({ s, update }: { s: Settings; update: Update }) {
  return (
    <>
      <div>
        <h1>通用</h1>
        <div className="lede">番茄和休息的长度。开始专注时也可以为这一轮单独选时长。</div>
      </div>
      <div className="rows">
        <Row name="专注时长" desc="开始专注时默认选中的时长">
          <NumberIn label="专注时长" value={s.focus_min} min={1} max={180} onChange={(n) => update({ focus_min: n })} />
          <span className="unit">分钟</span>
        </Row>
        <Row name="短休息" desc="每个番茄之后的休息">
          <NumberIn label="短休息" value={s.short_break_min} min={1} max={60} onChange={(n) => update({ short_break_min: n })} />
          <span className="unit">分钟</span>
        </Row>
        <Row name="长休息" desc={`每完成 ${s.long_break_every} 个番茄，换成一次长休息`}>
          <NumberIn label="长休息" value={s.long_break_min} min={1} max={120} onChange={(n) => update({ long_break_min: n })} />
          <span className="unit">分钟</span>
        </Row>
        <Row name="长休息间隔" desc="几个番茄之后长休息一次">
          <NumberIn label="长休息间隔" value={s.long_break_every} min={1} max={12} onChange={(n) => update({ long_break_every: n })} />
          <span className="unit">个</span>
        </Row>
        <Row name="提示音" desc="番茄结束和分心提醒时播放系统提示音">
          <input type="checkbox" className="switch" aria-label="提示音" checked={s.sound} onChange={(e) => update({ sound: e.target.checked })} />
        </Row>
        <Row name="回来后自动继续" desc="离开暂停后，一有键鼠操作就继续计时；关掉则等你点「继续专注」">
          <input type="checkbox" className="switch" aria-label="回来后自动继续" checked={s.auto_resume} onChange={(e) => update({ auto_resume: e.target.checked })} />
        </Row>
      </div>
    </>
  );
}

function ChainPane({ s, update, go }: { s: Settings; update: Update; go: (p: PaneId) => void }) {
  const r = s.rules;
  const blocked = r.block_apps.length + r.block_domains.length + r.block_keywords.length;
  const allowed = r.allow_apps.length + r.allow_domains.length + r.allow_keywords.length;
  const modelName = (m: ProviderConfig | null) => (m?.model ? m.model : "未配置");
  return (
    <>
      <div>
        <h1>判断流程</h1>
        <div className="lede">每次采样从上往下判断，哪一步有了结论就停。越往下越慢，也越花钱。</div>
      </div>
      <div className="rows">
        <Row step={1} name="采样" tag="local" desc="读取前台 App、窗口标题、URL 和空闲时间">
          <Choice label="采样间隔" value={s.sample_secs} options={[3, 4, 5, 10]} fmt={secs} onChange={(n) => update({ sample_secs: n })} />
        </Row>
        <Row step={2} name="空闲检测" tag="local" desc="键鼠无操作超过这个时间，视为离开：暂停计时，不算分心">
          <Choice label="空闲阈值" value={s.idle_secs} options={[60, 120, 300, 600]} fmt={span} onChange={(n) => update({ idle_secs: n })} />
        </Row>
        <Row step={3} name="规则匹配" tag="local" desc={`App 和域名命中名单就直接出结论 · 黑名单 ${blocked} 项，白名单 ${allowed} 项`}>
          <button className="btn small" style={{ fontSize: 13 }} onClick={() => go("lists")}>
            编辑名单…
          </button>
        </Row>
        <Row step={4} name="文本模型" tag="model" desc="规则没命中时，把标题、URL、窗口文字和任务描述发给模型">
          <button className="select" onClick={() => go("models")}>
            {modelName(s.text_model)}
          </button>
          <input type="checkbox" className="switch" aria-label="启用文本模型" checked={s.text_enabled} onChange={(e) => update({ text_enabled: e.target.checked })} />
        </Row>
        <Row step={5} name="视觉模型" tag="model" desc="文本模型没把握时才截图；画面没变（pHash 相同）不重复发">
          <button className="select" onClick={() => go("models")}>
            {modelName(s.vision_model)}
          </button>
          <input type="checkbox" className="switch" aria-label="启用视觉模型" checked={s.vision_enabled} onChange={(e) => update({ vision_enabled: e.target.checked })} />
        </Row>
        <Row step={6} name="提醒" tag="local" desc="连续几次采样都判为分心才弹提醒，避免切一下窗口就被打扰">
          <Choice label="连续分心次数" value={s.streak} options={[2, 3, 5]} fmt={(n) => `连续 ${n} 次`} onChange={(n) => update({ streak: n })} />
        </Row>
      </div>
      <div className="foot-note">
        <LockIcon />
        <span>关掉第 4、5 步后只用本地规则判断，任何内容都不会离开这台 Mac。</span>
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
        <span className="cap num">{value.length} 项</span>
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
        <h1>黑白名单</h1>
        <div className="lede">每行一项，命中就直接出结论，不调用模型。先看 App，再看网站，最后看标题关键词；同一层里黑名单优先。</div>
      </div>
      <div className="lists">
        <ListBox label="黑名单 App" value={s.rules.block_apps} onChange={set("block_apps")} />
        <ListBox label="白名单 App" value={s.rules.allow_apps} onChange={set("allow_apps")} />
        <ListBox label="黑名单网站" value={s.rules.block_domains} onChange={set("block_domains")} />
        <ListBox label="白名单网站" value={s.rules.allow_domains} onChange={set("allow_domains")} />
        <ListBox label="标题黑名单关键词" value={s.rules.block_keywords} onChange={set("block_keywords")} />
        <ListBox label="标题白名单关键词" value={s.rules.allow_keywords} onChange={set("allow_keywords")} />
      </div>
      <div className="hint">网站写域名即可，如 bilibili.com，会同时匹配它的子域名。开始专注时加的「本轮额外允许」只对那一轮有效，且优先于这里的黑名单。</div>
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
          {cfg && <span className={ready ? "status ok" : "status no"}>{ready ? "可用" : "未就绪"}</span>}
        </div>
        <input
          type="checkbox"
          className="switch"
          aria-label={`配置${title}`}
          checked={!!cfg}
          onChange={(e) => onChange(e.target.checked ? { kind: "anthropic", base_url: "", model: "" } : null)}
        />
      </div>
      {cfg && (
        <div className="pgrid">
          <label htmlFor={`${slot}-kind`}>服务商</label>
          <select id={`${slot}-kind`} className="select" value={cfg.kind} onChange={(e) => onChange({ ...cfg, kind: e.target.value as ProviderKind })}>
            {(Object.keys(KIND_LABEL) as ProviderKind[]).map((k) => (
              <option key={k} value={k}>
                {KIND_LABEL[k]}
              </option>
            ))}
          </select>
          <label>Base URL</label>
          <TextIn label="Base URL" value={cfg.base_url} placeholder={URL_HINT[cfg.kind]} onChange={(v) => onChange({ ...cfg, base_url: v })} />
          <label>模型</label>
          <TextIn label="模型" value={cfg.model} placeholder={MODEL_HINT[cfg.kind][slot]} onChange={(v) => onChange({ ...cfg, model: v })} />
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
                  placeholder={hasKey ? "已存在钥匙串，输入新值以替换" : "sk-..."}
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
                      setMsg({ ok: true, text: "已保存到钥匙串" });
                    } catch (e) {
                      setMsg({ ok: false, text: errText(e) });
                    }
                  }}
                >
                  保存
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
              {busy ? "测试中…" : "测试连接"}
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
        <h1>模型与密钥</h1>
        <div className="lede">通过 rig-core 调用。不配置也能用，只靠空闲检测和规则判断。API Key 存在 macOS 钥匙串。</div>
      </div>
      <div className="rows">
        <Provider slot="text" title="文本模型" cfg={s.text_model} status={status} onKeySaved={refresh} onChange={(c) => update({ text_model: c })} />
        <Provider slot="vision" title="视觉模型" cfg={s.vision_model} status={status} onKeySaved={refresh} onChange={(c) => update({ vision_model: c })} />
      </div>
      <div className="rows">
        <Row name="有把握阈值" desc="文本模型的把握低于它，就交给视觉模型（或不下结论）；Decision 模型低于它时视为没把握">
          <input
            type="range"
            aria-label="有把握阈值"
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
        <Row name="截图去重" desc="两张截图的 pHash 距离不超过它，就沿用上一次的判断，不再发送">
          <NumberIn label="截图去重阈值" value={s.phash_threshold} min={0} max={32} onChange={(n) => update({ phash_threshold: n })} />
        </Row>
      </div>
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
        <h1>隐私</h1>
        <div className="lede">规则和空闲检测都在本地完成。只有启用模型时，才会把下面写明的内容发出去。</div>
      </div>
      <div className="rows">
        <Row name="屏幕录制" desc="读取其他 App 的窗口标题、截图。没有它只能按 App 名判断。授权后需重启应用。">
          <span className={granted ? "status ok" : "status no"}>{granted == null ? "" : granted ? "已授权" : "未授权"}</span>
          {!granted && (
            <button
              className="btn small"
              onClick={async () => {
                await api.requestScreenRecording();
                await api.openPrivacy("screen");
              }}
            >
              去授权
            </button>
          )}
        </Row>
        <Row
          name="辅助功能"
          desc="规则没命中时，读取前台窗口里的部分文字和焦点控件交给模型，也能补上 Firefox 等浏览器的网址。不读密码框，不保存。"
        >
          {s.ax_enabled && <span className={axGranted ? "status ok" : "status no"}>{axGranted == null ? "" : axGranted ? "已授权" : "未授权"}</span>}
          {s.ax_enabled && !axGranted && (
            <button
              className="btn small"
              onClick={async () => {
                await api.requestAccessibility();
                await api.openPrivacy("accessibility");
              }}
            >
              去授权
            </button>
          )}
          <input type="checkbox" className="switch" aria-label="读取窗口文字" checked={s.ax_enabled} onChange={(e) => update({ ax_enabled: e.target.checked })} />
        </Row>
        <Row name="自动化" desc="读取 Safari、Chrome、Arc、Edge、Brave 当前标签页的 URL，每个浏览器首次会单独弹窗。Firefox 的网址由辅助功能读取。">
          <button className="btn small" onClick={() => api.openPrivacy("automation")}>
            打开设置
          </button>
        </Row>
        <Row name="发给文本模型" desc="前台 App 名、窗口标题、URL、这一轮的任务描述；开启辅助功能时，还有焦点控件和窗口里的部分文字（最多约 800 字）。" />
        <Row name="发给视觉模型" desc="前台窗口所在屏幕的截图（缩小后的 JPEG），以及同样的文字信息。画面没变时不重复发送。" />
        <Row name="本地数据" desc="设置和统计存在 ~/Library/Application Support/dev.aoba.pomosupervisor/，只在这台 Mac 上。" />
      </div>
      <div className="foot-note">
        <LockIcon />
        <span>在「判断流程」里关掉第 4、5 步后只用本地规则判断，任何内容都不会离开这台 Mac。</span>
      </div>
    </>
  );
}

export default function SettingsWindow() {
  const [pane, setPane] = useState<PaneId>("chain");
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
    load();
    const subs = [
      onWindowShown(load),
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
      <nav className="sidebar" aria-label="设置分类" data-tauri-drag-region>
        {PANES.map(([id, name]) => (
          <button key={id} aria-current={pane === id ? "page" : undefined} onClick={() => setPane(id)}>
            {name}
          </button>
        ))}
      </nav>
      <main className="pane" data-tauri-drag-region>
        {err && <div className="msg bad">保存失败：{err}</div>}
        {s && pane === "general" && <GeneralPane s={s} update={update} />}
        {s && pane === "chain" && <ChainPane s={s} update={update} go={setPane} />}
        {s && pane === "lists" && <ListsPane s={s} update={update} />}
        {s && pane === "models" && <ModelsPane s={s} update={update} />}
        {s && pane === "privacy" && <PrivacyPane s={s} update={update} />}
      </main>
    </div>
  );
}

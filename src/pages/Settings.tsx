import { useEffect, useState } from "react";
import { api, errText, type ModelStatus, type ProviderConfig, type ProviderKind, type Rules, type Settings, type Slot } from "../lib/api";

const KIND_LABEL: Record<ProviderKind, string> = { openai: "OpenAI 兼容", anthropic: "Anthropic", ollama: "Ollama（本地）" };
const URL_HINT: Record<ProviderKind, string> = {
  openai: "留空为 OpenAI 官方；其他需带 /v1，如 https://api.deepseek.com/v1",
  anthropic: "留空为官方地址",
  ollama: "留空为 http://localhost:11434",
};
const MODEL_HINT: Record<ProviderKind, Record<Slot, string>> = {
  openai: { text: "如 deepseek-chat / gpt-4o-mini", vision: "需支持图片输入的模型" },
  anthropic: { text: "如 claude-haiku-5-5", vision: "如 claude-haiku-5-5" },
  ollama: { text: "如 qwen2.5:7b", vision: "如 qwen2.5vl:7b" },
};

function Num({ label, value, onChange, min, max, step = 1, unit }: {
  label: string; value: number; onChange: (n: number) => void; min: number; max: number; step?: number; unit?: string;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <span className="num">
        <input type="number" value={value} min={min} max={max} step={step} onChange={(e) => onChange(Number(e.target.value))} />
        {unit && <em>{unit}</em>}
      </span>
    </label>
  );
}

function Toggle({ label, hint, value, onChange }: { label: string; hint?: string; value: boolean; onChange: (b: boolean) => void }) {
  return (
    <label className="field toggle">
      <span>
        {label}
        {hint && <small>{hint}</small>}
      </span>
      <input type="checkbox" checked={value} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}

function ListBox({ label, value, onChange, tone }: { label: string; value: string[]; onChange: (v: string[]) => void; tone: "block" | "allow" }) {
  const [text, setText] = useState(value.join("\n"));
  useEffect(() => setText(value.join("\n")), [value]);
  return (
    <label className={`listbox ${tone}`}>
      <span>{label}</span>
      <textarea
        rows={4}
        value={text}
        spellCheck={false}
        onChange={(e) => setText(e.target.value)}
        onBlur={() => onChange(text.split("\n").map((s) => s.trim()).filter(Boolean))}
      />
    </label>
  );
}

function ProviderForm({ slot, title, cfg, onChange, status, onKeySaved }: {
  slot: Slot; title: string; cfg: ProviderConfig | null; onChange: (c: ProviderConfig | null) => void; status: ModelStatus | null;
  onKeySaved: () => void;
}) {
  const [key, setKey] = useState("");
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const hasKey = slot === "text" ? status?.text_key : status?.vision_key;
  const buildErr = slot === "text" ? status?.text_error : status?.vision_error;

  return (
    <div className="provider">
      <Toggle
        label={title}
        value={!!cfg}
        onChange={(on) => onChange(on ? { kind: "anthropic", base_url: "", model: "" } : null)}
      />
      {cfg && (
        <>
          <label className="field">
            <span>服务商</span>
            <select value={cfg.kind} onChange={(e) => onChange({ ...cfg, kind: e.target.value as ProviderKind })}>
              {(Object.keys(KIND_LABEL) as ProviderKind[]).map((k) => (
                <option key={k} value={k}>{KIND_LABEL[k]}</option>
              ))}
            </select>
          </label>
          <label className="field col">
            <span>Base URL</span>
            <input value={cfg.base_url} placeholder={URL_HINT[cfg.kind]} onChange={(e) => onChange({ ...cfg, base_url: e.target.value })} />
          </label>
          <label className="field col">
            <span>模型</span>
            <input value={cfg.model} placeholder={MODEL_HINT[cfg.kind][slot]} onChange={(e) => onChange({ ...cfg, model: e.target.value })} />
          </label>
          {cfg.kind !== "ollama" && (
            <label className="field col">
              <span>API Key {hasKey ? <small className="ok">已保存在钥匙串</small> : <small>未设置</small>}</span>
              <span className="row">
                <input type="password" value={key} placeholder={hasKey ? "••••••（输入新值以替换）" : "sk-..."} onChange={(e) => setKey(e.target.value)} />
                <button
                  className="btn small"
                  disabled={!key}
                  onClick={async () => {
                    try {
                      await api.setApiKey(slot, key);
                      setKey("");
                      onKeySaved();
                      setMsg({ ok: true, text: "已保存" });
                    } catch (e) {
                      setMsg({ ok: false, text: errText(e) });
                    }
                  }}
                >
                  保存
                </button>
              </span>
            </label>
          )}
          <div className="row">
            <button
              className="btn small ghost"
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
            <small className="muted">先“保存设置”再测试</small>
          </div>
          {buildErr && !msg && <div className="msg bad">{buildErr}</div>}
          {msg && <div className={msg.ok ? "msg good" : "msg bad"}>{msg.text}</div>}
        </>
      )}
    </div>
  );
}

export default function SettingsPage() {
  const [s, setS] = useState<Settings | null>(null);
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [saved, setSaved] = useState<string>("");

  const refreshStatus = () => api.modelStatus().then(setStatus);
  useEffect(() => {
    api.getSettings().then(setS);
    refreshStatus();
  }, []);
  if (!s) return null;

  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => setS({ ...s, [k]: v });
  const setRule = (k: keyof Rules, v: string[]) => setS({ ...s, rules: { ...s.rules, [k]: v } });

  const save = async () => {
    try {
      setS(await api.saveSettings(s));
      await refreshStatus();
      setSaved("已保存");
    } catch (e) {
      setSaved(errText(e));
    }
    setTimeout(() => setSaved(""), 2500);
  };

  return (
    <div className="settings">
      <section className="card">
        <h3>计时</h3>
        <Num label="专注" value={s.focus_min} min={1} max={180} unit="分钟" onChange={(n) => set("focus_min", n)} />
        <Num label="短休息" value={s.short_break_min} min={1} max={60} unit="分钟" onChange={(n) => set("short_break_min", n)} />
        <Num label="长休息" value={s.long_break_min} min={1} max={120} unit="分钟" onChange={(n) => set("long_break_min", n)} />
        <Num label="每几个番茄长休息" value={s.long_break_every} min={1} max={12} unit="个" onChange={(n) => set("long_break_every", n)} />
        <Toggle label="提示音" value={s.sound} onChange={(b) => set("sound", b)} />
      </section>

      <section className="card">
        <h3>监督</h3>
        <Num label="采样间隔" value={s.sample_secs} min={2} max={60} unit="秒" onChange={(n) => set("sample_secs", n)} />
        <Num label="空闲多久算离开" value={s.idle_secs} min={10} max={3600} unit="秒" onChange={(n) => set("idle_secs", n)} />
        <Num label="连续几次分心才提醒" value={s.streak} min={1} max={20} unit="次" onChange={(n) => set("streak", n)} />
        <label className="field">
          <span>
            文本模型“有把握”阈值<small>低于它就交给视觉模型</small>
          </span>
          <span className="num">
            <input type="range" min={0.5} max={0.95} step={0.05} value={s.confidence} onChange={(e) => set("confidence", Number(e.target.value))} />
            <em>{Math.round(s.confidence * 100)}%</em>
          </span>
        </label>
        <Num label="截图去重阈值（pHash 距离）" value={s.phash_threshold} min={0} max={32} onChange={(n) => set("phash_threshold", n)} />
      </section>

      <section className="card">
        <h3>规则 <span className="muted small">每行一个，命中即定，不调用模型</span></h3>
        <div className="lists">
          <ListBox tone="block" label="黑名单应用" value={s.rules.block_apps} onChange={(v) => setRule("block_apps", v)} />
          <ListBox tone="allow" label="白名单应用" value={s.rules.allow_apps} onChange={(v) => setRule("allow_apps", v)} />
          <ListBox tone="block" label="黑名单域名" value={s.rules.block_domains} onChange={(v) => setRule("block_domains", v)} />
          <ListBox tone="allow" label="白名单域名" value={s.rules.allow_domains} onChange={(v) => setRule("allow_domains", v)} />
          <ListBox tone="block" label="标题黑名单关键词" value={s.rules.block_keywords} onChange={(v) => setRule("block_keywords", v)} />
          <ListBox tone="allow" label="标题白名单关键词" value={s.rules.allow_keywords} onChange={(v) => setRule("allow_keywords", v)} />
        </div>
      </section>

      <section className="card">
        <h3>模型 <span className="muted small">经 rig-core 调用；不配置则只用空闲 + 规则</span></h3>
        <ProviderForm slot="text" title="文本模型（标题 + URL + 任务）" cfg={s.text_model} status={status} onKeySaved={refreshStatus} onChange={(c) => set("text_model", c)} />
        <hr />
        <Toggle
          label="启用视觉判断"
          hint="文本模型没把握时，把整屏截图发给视觉模型"
          value={s.vision_enabled}
          onChange={(b) => set("vision_enabled", b)}
        />
        {s.vision_enabled && (
          <ProviderForm slot="vision" title="视觉模型" cfg={s.vision_model} status={status} onKeySaved={refreshStatus} onChange={(c) => set("vision_model", c)} />
        )}
      </section>

      <div className="savebar">
        <span className="muted">{saved}</span>
        <button className="btn primary" onClick={save}>
          保存设置
        </button>
      </div>
    </div>
  );
}

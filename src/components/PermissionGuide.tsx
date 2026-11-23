import { useEffect, useState } from "react";
import { api } from "../lib/api";

/** Shown until Screen Recording is granted (needed for window titles and screenshots). */
export default function PermissionGuide() {
  const [granted, setGranted] = useState<boolean | null>(null);

  const check = () => api.checkPermissions().then((p) => setGranted(p.screen_recording));
  useEffect(() => {
    check();
    const t = setInterval(check, 5000);
    return () => clearInterval(t);
  }, []);

  if (granted !== false) return null;
  return (
    <div className="perm">
      <div className="perm-title">需要“屏幕录制”权限</div>
      <p>没有它，读不到其他应用的窗口标题，也无法截图，监督只能靠应用名判断。授权后需重启应用。</p>
      <div className="row">
        <button
          className="btn small"
          onClick={async () => {
            await api.requestScreenRecording();
            await api.openPrivacy("screen");
          }}
        >
          去授权
        </button>
        <button className="btn small ghost" onClick={() => api.openPrivacy("automation")}>
          浏览器网址（自动化）
        </button>
      </div>
    </div>
  );
}

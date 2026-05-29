import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { AiSettingsPublic } from "../types";

const DEFAULT_ANTHROPIC_MODEL = "claude-sonnet-4-6";
const DEFAULT_OPENAI_MODEL = "gpt-4o-mini";

export default function Settings() {
  const [loaded, setLoaded] = useState<AiSettingsPublic | null>(null);
  const [provider, setProvider] = useState<"anthropic" | "openai">("anthropic");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [loading, setLoading] = useState(true);
  const [keyWarning, setKeyWarning] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const s = await invoke<AiSettingsPublic>("get_ai_settings");
      setLoaded(s);
      const prov = s.provider === "openai" ? "openai" : "anthropic";
      setProvider(prov);
      setModel(prov === "openai" ? (s.openaiModel || "") : (s.anthropicModel || ""));
      setApiKey("");
      setSaved(false);
      setKeyWarning(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const hasKeyForProvider = useMemo(() => {
    if (!loaded) return false;
    return provider === "openai" ? loaded.hasOpenaiKey : loaded.hasAnthropicKey;
  }, [loaded, provider]);

  function onProviderChange(next: "anthropic" | "openai") {
    if (!loaded) {
      setProvider(next);
      return;
    }
    setProvider(next);
    setModel(next === "openai" ? (loaded.openaiModel || "") : (loaded.anthropicModel || ""));
    setApiKey("");
    setKeyWarning(null);
  }

  async function onSave() {
    setError(null);
    setSaved(false);
    setKeyWarning(null);
    const trimmedKey = apiKey.trim();
    if (!trimmedKey && !hasKeyForProvider) {
      setKeyWarning(
        provider === "openai"
          ? "No OpenAI API key yet. Paste a key below, or set OPENAI_API_KEY in the environment."
          : "No Anthropic API key yet. Paste a key below, or set ANTHROPIC_API_KEY in the environment."
      );
    }
    try {
      await invoke("save_ai_settings", {
        input: {
          provider,
          model: model.trim(),
          apiKey: trimmedKey || null,
        },
      });
      setSaved(true);
      setApiKey("");
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  const modelPlaceholder =
    provider === "openai" ? DEFAULT_OPENAI_MODEL : DEFAULT_ANTHROPIC_MODEL;

  if (loading && !loaded) {
    return <p className="muted">Loading settings…</p>;
  }

  return (
    <div style={{ maxWidth: 560 }}>
      <p style={{ marginTop: 0 }}>
        <Link to="/dashboard">← Projects</Link>
      </p>
      <h2 style={{ marginBottom: "0.35rem" }}>Settings</h2>
      <p className="meta" style={{ marginTop: 0 }}>
        AI keys are stored in your local SQLite database (not the OS keychain). Prefer a dedicated key with usage limits.
      </p>

      {error && <div className="error-banner">{error}</div>}
      {keyWarning && <div className="error-banner">{keyWarning}</div>}
      {saved && <div className="card" style={{ borderColor: "#3d5a40", marginBottom: "1rem" }}>Saved</div>}

      <section className="card" style={{ display: "grid", gap: "0.75rem" }}>
        <label style={{ display: "grid", gap: "0.35rem" }}>
          <span className="meta">Provider</span>
          <select
            className="search"
            style={{ maxWidth: "100%" }}
            value={provider}
            onChange={(e) => onProviderChange(e.target.value === "openai" ? "openai" : "anthropic")}
          >
            <option value="anthropic">Anthropic (Claude)</option>
            <option value="openai">OpenAI</option>
          </select>
        </label>

        <label style={{ display: "grid", gap: "0.35rem" }}>
          <span className="meta">
            API key ({provider === "openai" ? "OpenAI" : "Anthropic"})
            {hasKeyForProvider ? " — one is already saved" : ""}
          </span>
          <input
            className="search"
            style={{ maxWidth: "100%" }}
            type="password"
            autoComplete="off"
            placeholder={hasKeyForProvider ? "Leave blank to keep saved key" : "Paste API key"}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
        </label>

        <label style={{ display: "grid", gap: "0.35rem" }}>
          <span className="meta">Model (optional — defaults from env or built-in default)</span>
          <input
            className="search"
            style={{ maxWidth: "100%" }}
            type="text"
            placeholder={modelPlaceholder}
            value={model}
            onChange={(e) => setModel(e.target.value)}
          />
        </label>

        <div>
          <button type="button" className="btn btn-primary" onClick={() => void onSave()}>
            Save
          </button>
        </div>
      </section>

      <p className="muted" style={{ fontSize: "0.85rem", marginTop: "1rem" }}>
        Writer profile: <Link to="/setup">Profile</Link>
      </p>
    </div>
  );
}

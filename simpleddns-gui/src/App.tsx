import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface FrontendProfile {
  id: string;
  name: string;
  provider_type: string;
  domain: string;
  enabled: boolean;
  enable_ipv4: boolean;
  enable_ipv6: boolean;
  provider_config: any;
  // Status fields
  ipv4: string;
  ipv6: string;
  last_update: string;
  status_message: string;
}

interface DdnsProfile {
  id: string;
  name: string;
  provider_type: string;
  domain: string;
  enabled: boolean;
  enable_ipv4: boolean;
  enable_ipv6: boolean;
  provider_config: any;
}

function App() {
  const [profiles, setProfiles] = useState<FrontendProfile[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [globalRunning, setGlobalRunning] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);

  // Modal State
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingId, setEditingId] = useState("");
  const [formName, setFormName] = useState("");
  const [formProvider, setFormProvider] = useState("cloudflare");
  const [formDomain, setFormDomain] = useState("");
  const [formSubdomain, setFormSubdomain] = useState("");
  const [formKey1, setFormKey1] = useState("");
  const [formKey2, setFormKey2] = useState("");
  const [formV4, setFormV4] = useState(true);
  const [formV6, setFormV6] = useState(false);

  const fetchProfiles = async () => {
    try {
      const data = await invoke<FrontendProfile[]>("get_profiles");
      setProfiles(data);
    } catch (error) {
      console.error("Failed to fetch profiles:", error);
    }
  };

  const fetchLogs = async () => {
    try {
      const data = await invoke<string[]>("get_logs");
      setLogs(data);
    } catch (error) {
      console.error("Failed to fetch logs:", error);
    }
  };

  const fetchGlobalRunning = async () => {
    try {
      const running = await invoke<boolean>("get_global_running");
      setGlobalRunning(running);
    } catch (error) {
      console.error("Failed to fetch global running:", error);
    }
  };

  useEffect(() => {
    fetchProfiles();
    fetchLogs();
    fetchGlobalRunning();

    const unlistenLog = listen<string>("log", (event) => {
      setLogs((prev) => [...prev, event.payload]);
    });

    const unlistenStatus = listen("status_update", () => {
      fetchProfiles();
    });

    return () => {
      unlistenLog.then((f) => f());
      unlistenStatus.then((f) => f());
    };
  }, []);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const toggleGlobalRunning = async () => {
    try {
      await invoke("toggle_global_running", { running: !globalRunning });
      setGlobalRunning(!globalRunning);
    } catch (error) {
      console.error("Failed to toggle global running:", error);
    }
  };

  const toggleProfile = async (id: string, enabled: boolean) => {
    try {
      await invoke("toggle_profile", { id, enabled });
      fetchProfiles();
    } catch (error) {
      console.error("Failed to toggle profile:", error);
    }
  };

  const deleteProfile = async (id: string) => {
    if (!confirm("Are you sure you want to delete this profile?")) return;
    try {
      await invoke("delete_profile", { id });
      fetchProfiles();
    } catch (error) {
      console.error("Failed to delete profile:", error);
    }
  };

  const triggerUpdate = async () => {
    try {
      await invoke("trigger_update");
    } catch (error) {
      console.error("Failed to trigger update:", error);
    }
  };

  // --- Form Logic ---

  const openModal = (profile?: FrontendProfile) => {
    if (profile) {
      setEditingId(profile.id);
      setFormName(profile.name);
      setFormProvider(profile.provider_type);
      setFormDomain(profile.domain);
      
      // Extract config
      const config = profile.provider_config;
      const sub = config.sub_domain || "";
      setFormSubdomain(sub);
      setFormV4(profile.enable_ipv4);
      setFormV6(profile.enable_ipv6);

      // Extract keys based on provider
      let k1 = "";
      let k2 = "";
      switch (profile.provider_type) {
        case "aliyun":
          k1 = config.access_key_id || "";
          k2 = config.access_key_secret || "";
          break;
        case "cloudflare":
          k1 = config.token || "";
          break;
        case "dnspod":
          const token = config.login_token || "";
          if (token.includes(",")) {
            [k1, k2] = token.split(",");
          } else {
            k1 = token;
          }
          break;
        case "godaddy":
          k1 = config.key || "";
          k2 = config.secret || "";
          break;
        case "namecheap":
          k1 = config.api_user || "";
          k2 = config.api_key || "";
          break;
      }
      setFormKey1(k1);
      setFormKey2(k2);
    } else {
      setEditingId("");
      setFormName("New Profile");
      setFormProvider("cloudflare");
      setFormDomain("");
      setFormSubdomain("");
      setFormKey1("");
      setFormKey2("");
      setFormV4(true);
      setFormV6(false);
    }
    setIsModalOpen(true);
  };

  const handleSave = async () => {
    // Build provider config
    const config: any = {
      zone_name: formDomain,
      sub_domain: formSubdomain,
    };

    switch (formProvider) {
      case "aliyun":
        config.access_key_id = formKey1;
        config.access_key_secret = formKey2;
        break;
      case "cloudflare":
        config.token = formKey1;
        break;
      case "dnspod":
        config.login_token = `${formKey1},${formKey2}`;
        break;
      case "godaddy":
        config.key = formKey1;
        config.secret = formKey2;
        break;
      case "namecheap":
        config.api_user = formKey1;
        config.api_key = formKey2;
        break;
    }

    const profile: DdnsProfile = {
      id: editingId, // Empty string for new
      name: formName,
      provider_type: formProvider,
      domain: formDomain,
      enabled: true, // Default enabled when saving/creating? Or preserve?
      enable_ipv4: formV4,
      enable_ipv6: formV6,
      provider_config: config,
    };

    // If editing, we might want to preserve the 'enabled' state, but the form doesn't expose it.
    // The backend `save_profile` overwrites.
    // So we should fetch the current enabled state if editing.
    if (editingId) {
      const existing = profiles.find(p => p.id === editingId);
      if (existing) {
        profile.enabled = existing.enabled;
      }
    }

    try {
      await invoke("save_profile", { profile });
      setIsModalOpen(false);
      fetchProfiles();
    } catch (error) {
      console.error("Failed to save profile:", error);
      alert("Failed to save profile: " + error);
    }
  };

  return (
    <div className="container">
      <header>
        <h1>SimpleDDNS</h1>
        <div className="controls">
          <label className="switch">
            <input 
              type="checkbox" 
              checked={globalRunning} 
              onChange={toggleGlobalRunning}
            />
            <span className="slider"></span>
          </label>
          <span>{globalRunning ? "Running" : "Stopped"}</span>
          <button className="btn btn-primary" onClick={triggerUpdate}>Trigger Update</button>
          <button className="btn btn-primary" onClick={() => openModal()}>Add Profile</button>
        </div>
      </header>

      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Domain</th>
              <th>Provider</th>
              <th>IPs</th>
              <th>Last Update</th>
              <th>Status</th>
              <th>Enabled</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {profiles.map((profile) => (
              <tr key={profile.id}>
                <td>{profile.name}</td>
                <td>{profile.domain}</td>
                <td>{profile.provider_type}</td>
                <td>
                  <div>v4: {profile.ipv4 || "-"}</div>
                  <div>v6: {profile.ipv6 || "-"}</div>
                </td>
                <td>{profile.last_update || "-"}</td>
                <td>
                  <span className="status-badge">{profile.status_message}</span>
                </td>
                <td>
                  <label className="switch">
                    <input 
                      type="checkbox" 
                      checked={profile.enabled} 
                      onChange={(e) => toggleProfile(profile.id, e.target.checked)}
                    />
                    <span className="slider"></span>
                  </label>
                </td>
                <td>
                  <button className="btn btn-secondary" style={{ marginRight: 5 }} onClick={() => openModal(profile)}>Edit</button>
                  <button className="btn btn-danger" onClick={() => deleteProfile(profile.id)}>Delete</button>
                </td>
              </tr>
            ))}
            {profiles.length === 0 && (
              <tr>
                <td colSpan={8} style={{ textAlign: "center", color: "#999" }}>No profiles found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="logs-container">
        {logs.map((log, index) => (
          <div key={index} className="log-entry">{log}</div>
        ))}
        <div ref={logsEndRef} />
      </div>

      {isModalOpen && (
        <div className="modal-overlay" onClick={(e) => { if(e.target === e.currentTarget) setIsModalOpen(false); }}>
          <div className="modal">
            <div className="modal-header">
              <h2>{editingId ? "Edit Profile" : "New Profile"}</h2>
              <button className="btn btn-secondary" onClick={() => setIsModalOpen(false)}>X</button>
            </div>
            
            <div className="form-group">
              <label>Name</label>
              <input value={formName} onChange={e => setFormName(e.target.value)} />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>Provider</label>
                <select value={formProvider} onChange={e => setFormProvider(e.target.value)}>
                  <option value="aliyun">Aliyun</option>
                  <option value="cloudflare">Cloudflare</option>
                  <option value="dnspod">Dnspod</option>
                  <option value="godaddy">GoDaddy</option>
                  <option value="namecheap">Namecheap</option>
                </select>
              </div>
            </div>

            <div className="form-row">
              <div className="form-group">
                <label>Domain (Zone)</label>
                <input value={formDomain} onChange={e => setFormDomain(e.target.value)} placeholder="example.com" />
              </div>
              <div className="form-group">
                <label>Subdomain</label>
                <input value={formSubdomain} onChange={e => setFormSubdomain(e.target.value)} placeholder="www, @, *" />
              </div>
            </div>

            {/* Dynamic fields based on provider */}
            {formProvider === "aliyun" && (
              <>
                <div className="form-group">
                  <label>Access Key ID</label>
                  <input value={formKey1} onChange={e => setFormKey1(e.target.value)} />
                </div>
                <div className="form-group">
                  <label>Access Key Secret</label>
                  <input value={formKey2} onChange={e => setFormKey2(e.target.value)} type="password" />
                </div>
              </>
            )}

            {formProvider === "cloudflare" && (
              <div className="form-group">
                <label>API Token</label>
                <input value={formKey1} onChange={e => setFormKey1(e.target.value)} type="password" />
              </div>
            )}

            {formProvider === "dnspod" && (
              <>
                <div className="form-group">
                  <label>ID</label>
                  <input value={formKey1} onChange={e => setFormKey1(e.target.value)} />
                </div>
                <div className="form-group">
                  <label>Token</label>
                  <input value={formKey2} onChange={e => setFormKey2(e.target.value)} type="password" />
                </div>
              </>
            )}

            {formProvider === "godaddy" && (
              <>
                <div className="form-group">
                  <label>Key</label>
                  <input value={formKey1} onChange={e => setFormKey1(e.target.value)} />
                </div>
                <div className="form-group">
                  <label>Secret</label>
                  <input value={formKey2} onChange={e => setFormKey2(e.target.value)} type="password" />
                </div>
              </>
            )}

            {formProvider === "namecheap" && (
              <>
                <div className="form-group">
                  <label>API User</label>
                  <input value={formKey1} onChange={e => setFormKey1(e.target.value)} />
                </div>
                <div className="form-group">
                  <label>API Key</label>
                  <input value={formKey2} onChange={e => setFormKey2(e.target.value)} type="password" />
                </div>
              </>
            )}

            <div className="checkbox-group">
              <label className="checkbox-label">
                <input type="checkbox" checked={formV4} onChange={e => setFormV4(e.target.checked)} />
                IPv4
              </label>
              <label className="checkbox-label">
                <input type="checkbox" checked={formV6} onChange={e => setFormV6(e.target.checked)} />
                IPv6
              </label>
            </div>

            <div className="modal-footer">
              <button className="btn btn-secondary" onClick={() => setIsModalOpen(false)}>Cancel</button>
              <button className="btn btn-primary" onClick={handleSave}>Save</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;

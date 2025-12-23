import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import {
  FolderOpen,
  Play,
  Settings,
  FileSpreadsheet,
  Check,
  AlertCircle,
  Loader2,
  ExternalLink,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

function App() {
  const [config, setConfig] = useState({
    company_name: "百惠行对账单",
    address: "东莞市黄江镇华南塑胶城区132号",
    phone: "(0769) 83631717",
    fax: "83637787",
    raw_data_path: "",
    output_path: "",
  });

  const [logs, setLogs] = useState([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [result, setResult] = useState(null);
  const logEndRef = useRef(null);

  useEffect(() => {
    loadConfig();
    const unlisten = listen("log", (event) => {
      addLog(event.payload, "info");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const loadConfig = async () => {
    try {
      const defaultConfig = await invoke("get_default_config");
      setConfig(defaultConfig);
    } catch (error) {
      console.error("加载配置失败:", error);
    }
  };

  const addLog = (message, level = "info") => {
    const timestamp = new Date().toLocaleTimeString();
    setLogs((prev) => [...prev, { message, level, timestamp }]);
  };

  const selectFolder = async (field) => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected) {
        setConfig((prev) => ({ ...prev, [field]: selected }));
      }
    } catch (error) {
      console.error("选择文件夹失败:", error);
      addLog(`选择文件夹失败: ${error}`, "error");
    }
  };

  const handleProcess = async () => {
    if (!config.raw_data_path || !config.output_path) {
      addLog("请先选择原始数据文件夹和输出文件夹", "error");
      return;
    }
    setIsProcessing(true);
    setLogs([]);
    setResult(null);
    try {
      addLog("开始处理送货单...", "info");
      const result = await invoke("process_delivery_orders", { config });
      setResult(result);
      addLog("处理完成！", "success");
    } catch (error) {
      console.error("处理失败:", error);
      addLog(`处理失败: ${error}`, "error");
    } finally {
      setIsProcessing(false);
    }
  };

  const openOutputFolder = async () => {
    if (config.output_path) {
      try {
        await openPath(config.output_path);
      } catch (error) {
        console.error("打开文件夹失败:", error);
        addLog(`打开文件夹失败: ${error}`, "error");
      }
    }
  };

  return (
    <div className="min-h-screen bg-slate-50">
      {/* Header */}
      <header className="bg-white border-b border-slate-200">
        <div className="max-w-3xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-emerald-500 rounded-lg flex items-center justify-center">
              <FileSpreadsheet className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-lg font-semibold text-slate-900">对账单生成器</h1>
              <p className="text-sm text-slate-500">批量处理送货单</p>
            </div>
          </div>
          <button
            onClick={() => setShowSettings(!showSettings)}
            className="p-2 hover:bg-slate-100 rounded-lg transition-colors"
          >
            <Settings className="w-5 h-5 text-slate-600" />
          </button>
        </div>
      </header>

      <main className="max-w-3xl mx-auto px-6 py-6 space-y-4">
        {/* Settings Panel */}
        {showSettings && (
          <div className="bg-white rounded-xl border border-slate-200 p-5">
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-medium text-slate-900">公司信息</h2>
              <button
                onClick={() => setShowSettings(false)}
                className="text-sm text-slate-500 hover:text-slate-700"
              >
                收起
              </button>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm text-slate-600 mb-1.5">公司名称</label>
                <input
                  type="text"
                  value={config.company_name}
                  onChange={(e) => setConfig({ ...config, company_name: e.target.value })}
                  className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                />
              </div>
              <div>
                <label className="block text-sm text-slate-600 mb-1.5">电话</label>
                <input
                  type="text"
                  value={config.phone}
                  onChange={(e) => setConfig({ ...config, phone: e.target.value })}
                  className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                />
              </div>
              <div className="col-span-2">
                <label className="block text-sm text-slate-600 mb-1.5">地址</label>
                <input
                  type="text"
                  value={config.address}
                  onChange={(e) => setConfig({ ...config, address: e.target.value })}
                  className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                />
              </div>
              <div>
                <label className="block text-sm text-slate-600 mb-1.5">传真</label>
                <input
                  type="text"
                  value={config.fax}
                  onChange={(e) => setConfig({ ...config, fax: e.target.value })}
                  className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                />
              </div>
            </div>
          </div>
        )}

        {/* Path Selection */}
        <div className="bg-white rounded-xl border border-slate-200 p-5 space-y-4">
          <div>
            <label className="block text-sm font-medium text-slate-700 mb-2">
              原始数据文件夹
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={config.raw_data_path}
                onChange={(e) => setConfig({ ...config, raw_data_path: e.target.value })}
                placeholder="选择包含送货单的文件夹..."
                className="flex-1 px-3 py-2.5 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent bg-slate-50"
              />
              <button
                onClick={() => selectFolder("raw_data_path")}
                className="px-4 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg flex items-center gap-2 text-sm font-medium transition-colors"
              >
                <FolderOpen className="w-4 h-4" />
                选择
              </button>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-slate-700 mb-2">
              输出文件夹
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={config.output_path}
                onChange={(e) => setConfig({ ...config, output_path: e.target.value })}
                placeholder="选择对账单输出位置..."
                className="flex-1 px-3 py-2.5 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent bg-slate-50"
              />
              <button
                onClick={() => selectFolder("output_path")}
                className="px-4 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg flex items-center gap-2 text-sm font-medium transition-colors"
              >
                <FolderOpen className="w-4 h-4" />
                选择
              </button>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-3">
          <button
            onClick={handleProcess}
            disabled={isProcessing}
            className="flex-1 py-3 bg-emerald-500 hover:bg-emerald-600 disabled:bg-slate-300 text-white rounded-xl flex items-center justify-center gap-2 font-medium transition-colors disabled:cursor-not-allowed"
          >
            {isProcessing ? (
              <>
                <Loader2 className="w-5 h-5 animate-spin" />
                处理中...
              </>
            ) : (
              <>
                <Play className="w-5 h-5" />
                开始生成
              </>
            )}
          </button>

          <button
            onClick={openOutputFolder}
            disabled={!config.output_path}
            className="px-5 py-3 border border-slate-300 hover:bg-slate-50 disabled:opacity-50 text-slate-700 rounded-xl flex items-center gap-2 font-medium transition-colors disabled:cursor-not-allowed"
          >
            <ExternalLink className="w-5 h-5" />
            打开输出
          </button>
        </div>

        {/* Result */}
        {result && (
          <div className="bg-emerald-50 border border-emerald-200 rounded-xl p-4 flex items-center gap-4">
            <div className="w-10 h-10 bg-emerald-500 rounded-full flex items-center justify-center flex-shrink-0">
              <Check className="w-5 h-5 text-white" />
            </div>
            <div className="flex-1">
              <p className="font-medium text-emerald-900">处理完成</p>
              <p className="text-sm text-emerald-700">
                生成 {result.generated_count} 个对账单，跳过 {result.skipped_count} 个
              </p>
            </div>
          </div>
        )}

        {/* Logs */}
        <div className="bg-white rounded-xl border border-slate-200 overflow-hidden">
          <div className="px-5 py-3 border-b border-slate-200 flex items-center justify-between">
            <span className="text-sm font-medium text-slate-700">运行日志</span>
            {logs.length > 0 && (
              <span className="text-xs text-slate-400">{logs.length} 条</span>
            )}
          </div>
          <div className="bg-slate-900 p-4 h-56 overflow-y-auto font-mono text-xs">
            {logs.length === 0 ? (
              <div className="h-full flex items-center justify-center text-slate-500">
                等待开始...
              </div>
            ) : (
              <div className="space-y-1">
                {logs.map((log, index) => (
                  <div
                    key={index}
                    className={`flex gap-3 ${
                      log.level === "error"
                        ? "text-red-400"
                        : log.level === "success"
                        ? "text-emerald-400"
                        : "text-slate-300"
                    }`}
                  >
                    <span className="text-slate-600 flex-shrink-0">{log.timestamp}</span>
                    <span className="flex-shrink-0">
                      {log.level === "error" && <AlertCircle className="w-3.5 h-3.5" />}
                      {log.level === "success" && <Check className="w-3.5 h-3.5" />}
                    </span>
                    <span>{log.message}</span>
                  </div>
                ))}
                <div ref={logEndRef} />
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}

export default App;

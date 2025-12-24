import { useState, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, ask } from "@tauri-apps/plugin-dialog";
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
  Users,
  Calendar,
  Search,
  ChevronRight,
  Database,
  ChevronUp,
  LayoutDashboard,
  List,
  TrendingUp,
  Award,
  BarChart3,
  Filter,
  PieChart
} from "lucide-react";

function App() {
  const [config, setConfig] = useState({
    company_name: "",
    address: "",
    phone: "",
    fax: "",
    raw_data_path: "",
    output_path: "",
  });

  const [logs, setLogs] = useState([]);
  const [showLogs, setShowLogs] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [result, setResult] = useState(null);
  const logEndRef = useRef(null);
  const isMounted = useRef(false);

  // Dashboard State
  const [currentView, setCurrentView] = useState("preview"); // 'preview' | 'analysis'
  const [dashboardData, setDashboardData] = useState({ map: {}, customers: [], allItems: [] });
  const [selectedCustomer, setSelectedCustomer] = useState(null);
  const [selectedMonth, setSelectedMonth] = useState(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [isLoadingData, setIsLoadingData] = useState(false);

  // Analysis State
  const [analysisTarget, setAnalysisTarget] = useState("all"); // 'all' or customerName

  // Success Modal State
  const [successModalOpen, setSuccessModalOpen] = useState(false);
  const [generatedFilePath, setGeneratedFilePath] = useState("");

  useEffect(() => {
    if (!isMounted.current) {
        loadConfig();
        isMounted.current = true;
    }
    const unlisten = listen("log", (event) => {
      addLog(event.payload, "info");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });

    // 如果最新的一条日志是错误或警告，自动展开日志面板
    if (logs.length > 0) {
      const lastLog = logs[logs.length - 1];
      if (lastLog.level === "error" || lastLog.level === "warning") {
        setShowLogs(true);
      }
    }
  }, [logs]);

  const loadConfig = async () => {
    try {
      const savedConfig = await invoke("load_config");
      setConfig(savedConfig);

      if (savedConfig.raw_data_path) {
        scanAndValidate(savedConfig);
      }
    } catch (error) {
      console.error("加载配置失败:", error);
    }
  };

  const processData = (items) => {
    const map = {};
    const customerSet = new Set();

    items.forEach((item) => {
      if (!item.customer) return;
      customerSet.add(item.customer);

      // 解析年月 YYYY-MM
      let month = "未知";
      try {
        const dateParts = item.date.split(/[-/]/);
        if (dateParts.length >= 2) {
          month = `${dateParts[0]}年${parseInt(dateParts[1])}月`;
        } else {
          month = "其他日期";
        }
      } catch (e) {
        month = "日期格式错误";
      }

      if (!map[item.customer]) map[item.customer] = {};
      if (!map[item.customer][month]) map[item.customer][month] = [];
      map[item.customer][month].push(item);
    });

    return {
      map,
      customers: Array.from(customerSet).sort(),
      allItems: items,
    };
  };

  const scanAndValidate = async (currentConfig) => {
    setIsLoadingData(true);
    try {
      addLog("正在自动扫描并验证原始数据...", "info");
      const result = await invoke("scan_and_validate", { config: currentConfig });

      if (result.success) {
        addLog(`数据验证通过: 找到 ${result.total_files} 个文件`, "success");
      } else {
        addLog(`数据验证发现问题: ${result.message}`, "error");
        result.errors.forEach((err) => {
          const fileName = err.file.split(/[/\\]/).pop();
          addLog(`${fileName}: ${err.error}`, "error");
        });
      }

      if (result.warnings && result.warnings.length > 0) {
        addLog(`注意: 发现 ${result.warnings.length} 个文件存在警告⚠️`, "warning");
        result.warnings.forEach((warn) => {
          const fileName = warn.file.split(/[/\\]/).pop();
          addLog(`${fileName}: ${warn.error}`, "warning");
        });
      }

      // 如果有错误或警告，自动展开日志面板
      if (!result.success || (result.warnings && result.warnings.length > 0)) {
        setShowLogs(true);
      }

      // 处理用于显示的数据
      if (result.items) {
        const processed = processData(result.items);
        setDashboardData(processed);
        if (processed.customers.length > 0 && !selectedCustomer) {
            // 默认不用选中，或者可以选中第一个
            // setSelectedCustomer(processed.customers[0]);
        }
      }

    } catch (error) {
      console.error("扫描失败:", error);
      addLog(`扫描失败: ${error}`, "error");
    } finally {
      setIsLoadingData(false);
    }
  };

  const saveConfig = async (newConfig) => {
    setConfig(newConfig);
    try {
      await invoke("save_config", { config: newConfig });
      // 配置变更后重新扫描
      if (newConfig.raw_data_path !== config.raw_data_path) {
          scanAndValidate(newConfig);
      }
    } catch (error) {
      console.error("保存配置失败:", error);
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
        const newConfig = { ...config, [field]: selected };
        saveConfig(newConfig);
      }
    } catch (error) {
      console.error("选择文件夹失败:", error);
      addLog(`选择文件夹失败: ${error}`, "error");
    }
  };

  const handleGenerateSingle = async () => {
    if (!selectedCustomer || !selectedMonth) return;

    // 检查配置
    if (!config.output_path) {
        addLog("请先配置输出文件夹", "error");
        setShowSettings(true);
        return;
    }

    const items = dashboardData.map[selectedCustomer]?.[selectedMonth] || [];
    if (items.length === 0) {
        addLog("当前选择无数据", "error");
        return;
    }

    setIsProcessing(true);

    const callGenerate = async (overwrite = false) => {
        const result = await invoke("generate_single_statement", {
            config,
            items,
            customer: selectedCustomer,
            month: selectedMonth,
            overwrite
        });

        if (result.success) {
            addLog(result.message, "success");
            const filePath = result.message.replace("已生成: ", "");
            setGeneratedFilePath(filePath);
            setSuccessModalOpen(true);
        }
    };

    try {
      addLog(`开始生成对账单: ${selectedCustomer} ${selectedMonth}...`, "info");
      await callGenerate(false);
    } catch (error) {
      // Normalize error message
      const errorStr = typeof error === 'string' ? error : (error?.message || JSON.stringify(error));

      if (errorStr.includes("FILE_EXISTS")) {
          const confirmed = await ask(`对账单已存在，是否覆盖？\n\n客户: ${selectedCustomer}\n月份: ${selectedMonth}`, {
              title: "确认覆盖",
              type: "warning",
              okLabel: "覆盖",
              cancelLabel: "取消"
          });

          if (confirmed) {
              try {
                  await callGenerate(true);
              } catch (retryError) {
                  console.error("覆盖生成失败:", retryError);
                  addLog(`覆盖生成失败: ${retryError}`, "error");
              }
          } else {
              addLog("已取消生成", "info");
          }
      } else {
          console.error("生成失败:", error);
          addLog(`生成失败: ${errorStr}`, "error");
      }
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

  const openFile = async (path) => {
    if (!path) return;
    try {
      addLog(`尝试打开文件: ${path}`, "info");
      await openPath(path);
    } catch (error) {
      console.error("打开文件失败:", error);
      addLog(`打开文件失败: ${error}`, "error");
    }
  };

  const filteredCustomers = useMemo(() => {
    if (!searchTerm) return dashboardData.customers;
    return dashboardData.customers.filter(c =>
      c.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [dashboardData.customers, searchTerm]);

  // 获取当前选中客户的所有月份，并排序
  const customerMonths = useMemo(() => {
    if (!selectedCustomer || !dashboardData.map[selectedCustomer]) return [];
    return Object.keys(dashboardData.map[selectedCustomer]).sort((a, b) => {
      // 简单排序：年份大的在前，月份大的在前
      // 格式：2024年1月
      const parse = (s) => {
        const parts = s.match(/(\d+)年(\d+)月/);
        if (parts) return parseInt(parts[1]) * 100 + parseInt(parts[2]);
        return 0;
      };
      return parse(b) - parse(a);
    });
  }, [selectedCustomer, dashboardData.map]);

  // 自动选择第一个月份
  useEffect(() => {
    if (customerMonths.length > 0) {
      setSelectedMonth(customerMonths[0]);
    } else {
        setSelectedMonth(null);
    }
  }, [customerMonths]);

  const currentItems = useMemo(() => {
    if (!selectedCustomer || !selectedMonth) return [];
    return dashboardData.map[selectedCustomer]?.[selectedMonth] || [];
  }, [selectedCustomer, selectedMonth, dashboardData.map]);

  const currentSummary = useMemo(() => {
    return currentItems.reduce((acc, item) => ({
      quantity: acc.quantity + item.quantity,
      amount: acc.amount + item.amount
    }), { quantity: 0, amount: 0 });
  }, [currentItems]);

  // --- 分析数据计算 ---
  const analysisData = useMemo(() => {
    if (!dashboardData.allItems || dashboardData.allItems.length === 0) return null;

    // 1. 过滤数据
    const items = analysisTarget === 'all'
        ? dashboardData.allItems
        : dashboardData.allItems.filter(i => i.customer === analysisTarget);

    if (items.length === 0) return null;

    const totalAmount = items.reduce((sum, item) => sum + item.amount, 0);
    const totalQuantity = items.reduce((sum, item) => sum + item.quantity, 0);
    const totalOrders = items.length; // 这里的Orders其实是Item粒度，姑且当作条目数

        // 2. 趋势数据 (按月聚合)
        const monthlyStats = {};
        items.forEach(item => {
            let key = "未知";
            // 尝试匹配 YYYY-MM-DD, YYYY/MM/DD, YYYY.MM.DD 等格式
            const match = item.date.match(/^(\d{4})[-/.](\d{1,2})[-/.]\d{1,2}/);
            if (match) {
                key = `${match[1]}-${match[2].padStart(2, '0')}`;
            } else {
                 // 尝试匹配 YYYY年MM月DD日
                 const cnMatch = item.date.match(/^(\d{4})年(\d{1,2})月/);
                 if (cnMatch) {
                     key = `${cnMatch[1]}-${cnMatch[2].padStart(2, '0')}`;
                 }
            }

            if (!monthlyStats[key]) monthlyStats[key] = 0;
            monthlyStats[key] += item.amount;
        });
        const monthlyTrend = Object.entries(monthlyStats)
            .map(([label, value]) => ({ label, value }))
            .sort((a, b) => a.label.localeCompare(b.label));
        // 3. 排行数据
    let rankingTitle = "";
    let rankingData = [];
    let rankingType = ""; // 'customer' or 'product'

    if (analysisTarget === 'all') {
        rankingTitle = "客户贡献排行 (Top 10)";
        rankingType = 'customer';
        const customerMap = {};
        items.forEach(item => {
            if(!customerMap[item.customer]) customerMap[item.customer] = 0;
            customerMap[item.customer] += item.amount;
        });
        rankingData = Object.entries(customerMap)
            .map(([name, value]) => ({name, value}))
            .sort((a,b) => b.value - a.value)
            .slice(0, 10);
    } else {
        rankingTitle = "热销产品排行 (Top 10)";
        rankingType = 'product';
        const productMap = {};
        items.forEach(item => {
            if(!productMap[item.product_name]) productMap[item.product_name] = { amount: 0, quantity: 0 };
            productMap[item.product_name].amount += item.amount;
            productMap[item.product_name].quantity += item.quantity;
        });
        rankingData = Object.entries(productMap)
            .map(([name, stats]) => ({name, value: stats.amount, quantity: stats.quantity}))
            .sort((a,b) => b.value - a.value)
            .slice(0, 10);
    }

    return { totalAmount, totalQuantity, totalOrders, monthlyTrend, rankingTitle, rankingData, rankingType };
  }, [dashboardData.allItems, analysisTarget]);


  return (
    <div className="h-screen bg-slate-50 flex flex-col">
      {/* Header */}
      <header className="bg-white border-b border-slate-200 flex-shrink-0 z-10">
        <div className="px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-emerald-500 rounded-lg flex items-center justify-center">
              <FileSpreadsheet className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-base font-semibold text-slate-900 leading-none">百惠行</h1>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setShowSettings(!showSettings)}
              className={`p-2 rounded-lg transition-colors ${
                showSettings ? "bg-emerald-100 text-emerald-600" : "hover:bg-slate-100 text-slate-600"
              }`}
            >
              <Settings className="w-5 h-5" />
            </button>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden relative">

        {/* Settings Overlay/Panel */}
        {showSettings && (
          <div className="absolute inset-0 bg-white/50 backdrop-blur-sm z-20 flex justify-end">
             <div className="w-96 h-full bg-white shadow-xl border-l border-slate-200 overflow-y-auto p-6 animate-in slide-in-from-right duration-200">
                <div className="flex items-center justify-between mb-6">
                    <h2 className="text-lg font-semibold text-slate-900">设置</h2>
                    <button onClick={() => setShowSettings(false)} className="text-slate-500 hover:text-slate-700">
                        <ChevronRight className="w-5 h-5" />
                    </button>
                </div>

                <div className="space-y-6">
                  {/* 路径配置 */}
                  <div>
                    <h3 className="text-sm font-medium text-slate-900 mb-3">文件路径</h3>
                    <div className="space-y-3">
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">原始数据文件夹</label>
                        <div className="flex gap-2">
                          <input
                            type="text"
                            value={config.raw_data_path}
                            readOnly
                            placeholder="点击选择..."
                            className="flex-1 px-3 py-2 border border-slate-300 rounded-lg text-sm bg-slate-50 cursor-pointer"
                            onClick={() => selectFolder("raw_data_path")}
                          />
                          <button
                            onClick={() => selectFolder("raw_data_path")}
                            className="px-3 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg flex items-center gap-1.5 text-sm transition-colors"
                          >
                            <FolderOpen className="w-4 h-4" />
                          </button>
                        </div>
                      </div>
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">输出文件夹</label>
                        <div className="flex gap-2">
                          <input
                            type="text"
                            value={config.output_path}
                            readOnly
                            placeholder="点击选择..."
                            className="flex-1 px-3 py-2 border border-slate-300 rounded-lg text-sm bg-slate-50 cursor-pointer"
                            onClick={() => selectFolder("output_path")}
                          />
                          <button
                            onClick={() => selectFolder("output_path")}
                            className="px-3 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg flex items-center gap-1.5 text-sm transition-colors"
                          >
                            <FolderOpen className="w-4 h-4" />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* 公司信息 */}
                  <div>
                    <h3 className="text-sm font-medium text-slate-900 mb-3">公司信息</h3>
                    <div className="grid grid-cols-1 gap-3">
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">公司名称</label>
                        <input
                          type="text"
                          value={config.company_name}
                          onChange={(e) => setConfig({ ...config, company_name: e.target.value })}
                          onBlur={() => saveConfig(config)}
                          className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                        />
                      </div>
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">电话</label>
                        <input
                          type="text"
                          value={config.phone}
                          onChange={(e) => setConfig({ ...config, phone: e.target.value })}
                          onBlur={() => saveConfig(config)}
                          className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                        />
                      </div>
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">地址</label>
                        <input
                          type="text"
                          value={config.address}
                          onChange={(e) => setConfig({ ...config, address: e.target.value })}
                          onBlur={() => saveConfig(config)}
                          className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                        />
                      </div>
                      <div>
                        <label className="block text-sm text-slate-600 mb-1.5">传真</label>
                        <input
                          type="text"
                          value={config.fax}
                          onChange={(e) => setConfig({ ...config, fax: e.target.value })}
                          onBlur={() => saveConfig(config)}
                          className="w-full px-3 py-2 border border-slate-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                        />
                      </div>
                    </div>
                  </div>

                   <button
                    onClick={openOutputFolder}
                    disabled={!config.output_path}
                    className="w-full px-5 py-3 border border-slate-300 hover:bg-slate-50 disabled:opacity-50 text-slate-700 rounded-xl flex items-center justify-center gap-2 font-medium transition-colors disabled:cursor-not-allowed"
                  >
                    <ExternalLink className="w-5 h-5" />
                    打开输出目录
                  </button>
                </div>
             </div>
          </div>
        )}

        {/* Sidebar */}
        <div className="w-64 bg-white border-r border-slate-200 flex flex-col">
          {/* Navigation Tabs */}
          <div className="flex border-b border-slate-200">
            <button
              onClick={() => setCurrentView("preview")}
              className={`flex-1 py-3 text-sm font-medium flex items-center justify-center gap-2 transition-colors ${
                currentView === "preview"
                  ? "text-emerald-600 border-b-2 border-emerald-500 bg-emerald-50/50"
                  : "text-slate-500 hover:bg-slate-50"
              }`}
            >
              <List className="w-4 h-4" />
              数据预览
            </button>
            <button
              onClick={() => setCurrentView("analysis")}
              className={`flex-1 py-3 text-sm font-medium flex items-center justify-center gap-2 transition-colors ${
                currentView === "analysis"
                  ? "text-emerald-600 border-b-2 border-emerald-500 bg-emerald-50/50"
                  : "text-slate-500 hover:bg-slate-50"
              }`}
            >
              <BarChart3 className="w-4 h-4" />
              经营分析
            </button>
          </div>

          {currentView === "preview" && (
            <>
              <div className="p-4 border-b border-slate-200">
                 <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                    <input
                      type="text"
                      placeholder="搜索客户..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="w-full pl-9 pr-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500"
                    />
                 </div>
              </div>
              <div className="flex-1 overflow-y-auto">
                 {isLoadingData ? (
                     <div className="p-8 text-center text-slate-400 text-sm">
                         <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2" />
                         加载数据中...
                     </div>
                 ) : filteredCustomers.length === 0 ? (
                     <div className="p-8 text-center text-slate-400 text-sm">
                         无客户数据
                     </div>
                 ) : (
                    <div className="divide-y divide-slate-100">
                      {filteredCustomers.map(customer => (
                        <button
                          key={customer}
                          onClick={() => setSelectedCustomer(customer)}
                          className={`w-full text-left px-4 py-3 text-sm hover:bg-slate-50 transition-colors flex items-center gap-3 ${
                            selectedCustomer === customer ? "bg-emerald-50 text-emerald-700 font-medium" : "text-slate-700"
                          }`}
                        >
                          <div className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                             selectedCustomer === customer ? "bg-emerald-100" : "bg-slate-100 text-slate-400"
                          }`}>
                             <Users className="w-4 h-4" />
                          </div>
                          <span className="truncate">{customer}</span>
                        </button>
                      ))}
                    </div>
                 )}
              </div>

              <div className="p-3 border-t border-slate-200 bg-slate-50">
                 <div className="flex items-center gap-2 text-xs text-slate-500">
                    <Database className="w-3 h-3" />
                    <span>共 {dashboardData.customers.length} 个客户</span>
                 </div>
              </div>
            </>
          )}

          {currentView === "analysis" && (
            <>
              <div className="p-4 border-b border-slate-200">
                 <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                    <input
                      type="text"
                      placeholder="搜索客户..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="w-full pl-9 pr-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500"
                    />
                 </div>
              </div>
              <div className="flex-1 overflow-y-auto">
                 {isLoadingData ? (
                     <div className="p-8 text-center text-slate-400 text-sm">
                         <Loader2 className="w-6 h-6 animate-spin mx-auto mb-2" />
                         加载数据中...
                     </div>
                 ) : (
                    <div className="divide-y divide-slate-100">
                      {/* Global Option */}
                      <button
                          onClick={() => setAnalysisTarget("all")}
                          className={`w-full text-left px-4 py-3 text-sm hover:bg-slate-50 transition-colors flex items-center gap-3 ${
                            analysisTarget === "all" ? "bg-emerald-50 text-emerald-700 font-medium" : "text-slate-700"
                          }`}
                        >
                          <div className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                             analysisTarget === "all" ? "bg-emerald-100" : "bg-slate-100 text-slate-400"
                          }`}>
                             <LayoutDashboard className="w-4 h-4" />
                          </div>
                          <span className="truncate">全公司</span>
                      </button>

                      {/* Customer List */}
                      {filteredCustomers.length === 0 ? (
                        <div className="p-8 text-center text-slate-400 text-sm">
                            无匹配客户
                        </div>
                      ) : (
                        filteredCustomers.map(customer => (
                            <button
                            key={customer}
                            onClick={() => setAnalysisTarget(customer)}
                            className={`w-full text-left px-4 py-3 text-sm hover:bg-slate-50 transition-colors flex items-center gap-3 ${
                                analysisTarget === customer ? "bg-emerald-50 text-emerald-700 font-medium" : "text-slate-700"
                            }`}
                            >
                            <div className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                                analysisTarget === customer ? "bg-emerald-100" : "bg-slate-100 text-slate-400"
                            }`}>
                                <Users className="w-4 h-4" />
                            </div>
                            <span className="truncate">{customer}</span>
                            </button>
                        ))
                      )}
                    </div>
                 )}
              </div>

              <div className="p-3 border-t border-slate-200 bg-slate-50">
                 <div className="flex items-center gap-2 text-xs text-slate-500">
                    <Database className="w-3 h-3" />
                    <span>共 {dashboardData.customers.length} 个客户</span>
                 </div>
              </div>
            </>
          )}
        </div>

        {/* Right Content */}
        <div className="flex-1 flex flex-col bg-slate-50 overflow-hidden">
           {currentView === "preview" ? (
               !selectedCustomer ? (
                 <div className="flex-1 flex flex-col items-center justify-center text-slate-400">
                    <Users className="w-16 h-16 mb-4 opacity-20" />
                    <p>请从左侧选择一个客户查看明细</p>
                 </div>
               ) : (
                 <>
                                      {/* Month Tabs & Actions */}
                                      <div className="bg-white border-b border-slate-200 px-6 py-2 flex items-center justify-between gap-4">
                                         <div className="flex gap-2 overflow-x-auto scrollbar-hide">
                                             {customerMonths.length === 0 ? (
                                                 <span className="text-sm text-slate-400 py-2">该客户暂无月份数据</span>
                                             ) : (
                                                 customerMonths.map(month => (
                                                   <button
                                                     key={month}
                                                     onClick={() => setSelectedMonth(month)}
                                                     className={`px-4 py-1.5 rounded-full text-sm font-medium transition-colors whitespace-nowrap ${
                                                       selectedMonth === month
                                                         ? "bg-slate-900 text-white"
                                                         : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                                                     }`}
                                                   >
                                                     {month}
                                                   </button>
                                                 ))
                                             )}
                                         </div>

                                         <div className="flex-shrink-0">
                                            <button
                                             onClick={handleGenerateSingle}
                                             disabled={isProcessing || !selectedMonth}
                                             className="px-4 py-1.5 bg-emerald-500 hover:bg-emerald-600 disabled:bg-slate-300 text-white rounded-lg flex items-center gap-2 text-sm font-medium transition-colors"
                                           >
                                             {isProcessing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
                                             {isProcessing ? "生成中..." : "生成当前对账单"}
                                           </button>
                                         </div>
                                      </div>
                                      {/* Data Table */}
                   <div className="flex-1 overflow-auto p-6">
                     <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
                       <div className="overflow-x-auto">
                        <table className="w-full text-sm text-left">
                          <thead className="bg-slate-50 border-b border-slate-200 text-slate-500 font-medium">
                            <tr>
                              <th className="px-4 py-3 w-32">日期</th>
                              <th className="px-4 py-3 w-32">送货单号</th>
                              <th className="px-4 py-3">货名</th>
                              <th className="px-4 py-3 w-24">规格</th>
                              <th className="px-4 py-3 w-20 text-right">数量</th>
                              <th className="px-4 py-3 w-16 text-center">单位</th>
                              <th className="px-4 py-3 w-24 text-right">单价</th>
                              <th className="px-4 py-3 w-30 text-right">金额</th>
                              <th className="px-4 py-3 w-42 text-slate-400 font-normal">来源</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-slate-100">
                            {currentItems.length === 0 ? (
                              <tr>
                                <td colSpan="9" className="px-4 py-8 text-center text-slate-400">
                                  无数据
                                </td>
                              </tr>
                            ) : (
                              currentItems.map((item, idx) => (
                                <tr key={idx} className="hover:bg-slate-50">
                                  <td className="px-4 py-3 text-slate-600">{item.date}</td>
                                  <td className="px-4 py-3 text-slate-600">{item.delivery_order_no}</td>
                                  <td
                                    className="px-4 py-3 text-slate-900 font-medium truncate max-w-[200px]"
                                    title={item.product_name}
                                  >
                                    {item.product_name}
                                  </td>
                                  <td className="px-4 py-3 text-slate-600">{item.spec}</td>
                                  <td className="px-4 py-3 text-right text-slate-900">{item.quantity}</td>
                                  <td className="px-4 py-3 text-center text-slate-600">{item.unit}</td>
                                  <td className="px-4 py-3 text-right text-slate-600">¥{item.unit_price.toFixed(2)}</td>
                                  <td className="px-4 py-3 text-right text-slate-900 font-medium">¥{item.amount.toFixed(2)}</td>
                                  <td
                                    className="px-4 py-3 text-emerald-600 text-xs truncate max-w-[150px] cursor-pointer hover:underline"
                                    title={item.source_file}
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      openFile(item.source_file);
                                    }}
                                  >
                                    {item.source_file.split(/[/\\]/).pop()}
                                  </td>
                                </tr>
                              ))
                            )}
                          </tbody>
                          {currentItems.length > 0 && (
                            <tfoot className="bg-slate-50 border-t border-slate-200 font-semibold text-slate-900">
                              <tr>
                                <td colSpan="4" className="px-4 py-3 text-right">本月合计:</td>
                                <td className="px-4 py-3 text-right">{currentSummary.quantity}</td>
                                <td className="px-4 py-3"></td>
                                <td className="px-4 py-3 text-right"></td>
                                <td className="px-4 py-3 text-right">¥{currentSummary.amount.toFixed(2)}</td>
                                <td className="px-4 py-3"></td>
                              </tr>
                            </tfoot>
                          )}
                        </table>
                       </div>
                     </div>
                   </div>
                 </>
               )
           ) : (
             // Analysis View
             <div className="flex-1 overflow-auto p-6 space-y-6">
                <div className="flex items-center justify-between mb-2">
                    <h2 className="text-xl font-bold text-slate-900 flex items-center gap-2">
                       {analysisTarget === 'all' ? "公司经营总览" : `客户分析: ${analysisTarget}`}
                    </h2>
                    <div className="text-sm text-slate-500">
                        数据范围: {analysisTarget === 'all' ? "所有客户" : "单一客户"}
                    </div>
                </div>

                {!analysisData ? (
                   <div className="text-center text-slate-400 mt-20">暂无数据可分析</div>
                ) : (
                  <>
                    {/* Summary Cards */}
                    <div className="grid grid-cols-3 gap-6">
                      <div className="bg-white p-5 rounded-xl border border-slate-200 shadow-sm">
                        <div className="flex items-center gap-3 text-slate-500 mb-2">
                          <div className="p-2 bg-emerald-100 rounded-lg text-emerald-600">
                            <TrendingUp className="w-5 h-5" />
                          </div>
                          <span className="text-sm font-medium">总销售额</span>
                        </div>
                        <div className="text-2xl font-bold text-slate-900">
                           ¥{analysisData.totalAmount.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2})}
                        </div>
                      </div>
                      <div className="bg-white p-5 rounded-xl border border-slate-200 shadow-sm">
                        <div className="flex items-center gap-3 text-slate-500 mb-2">
                          <div className="p-2 bg-blue-100 rounded-lg text-blue-600">
                            <List className="w-5 h-5" />
                          </div>
                          <span className="text-sm font-medium">总数据条目</span>
                        </div>
                        <div className="text-2xl font-bold text-slate-900">
                           {analysisData.totalOrders} <span className="text-sm font-normal text-slate-500">条</span>
                        </div>
                      </div>
                      <div className="bg-white p-5 rounded-xl border border-slate-200 shadow-sm">
                        <div className="flex items-center gap-3 text-slate-500 mb-2">
                          <div className="p-2 bg-purple-100 rounded-lg text-purple-600">
                            {analysisTarget === 'all' ? <Users className="w-5 h-5" /> : <Award className="w-5 h-5" />}
                          </div>
                          <span className="text-sm font-medium">{analysisTarget === 'all' ? "活跃客户" : "贡献排名"}</span>
                        </div>
                        <div className="text-2xl font-bold text-slate-900">
                           {analysisTarget === 'all'
                              ? <span>{dashboardData.customers.length} <span className="text-sm font-normal text-slate-500">家</span></span>
                              : <span className="text-base text-slate-500 font-normal">Top Clients</span>
                           }
                        </div>
                      </div>
                    </div>

                                        {/* Trend Chart */}
                                        <div className="bg-white p-6 rounded-xl border border-slate-200 shadow-sm">
                                            <div className="flex items-center gap-2 font-medium text-slate-900 mb-6">
                                                <TrendingUp className="w-4 h-4 text-emerald-500" />
                                                月度销售趋势
                                            </div>
                                            {analysisData.monthlyTrend.length === 0 ? (
                                                <div className="h-48 flex items-center justify-center text-slate-400 text-sm">
                                                    暂无月度数据
                                                </div>
                                            ) : (
                                                <div className="flex items-end gap-2 h-48">
                                                  {(() => {
                                                    const maxVal = Math.max(...analysisData.monthlyTrend.map(d => d.value)) || 1;
                                                    return analysisData.monthlyTrend.map((item, idx) => (
                                                        <div key={idx} className="flex-1 flex flex-col items-center group relative h-full justify-end">
                                                            {/* Tooltip */}
                                                            <div className="absolute bottom-full mb-2 bg-slate-800 text-white text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-10 pointer-events-none">
                                                                {item.label}: ¥{item.value.toLocaleString()}
                                                            </div>
                                                            <div className="w-full flex-1 flex items-end justify-center">
                                                                 <div
                                                                    className="w-full bg-emerald-100 hover:bg-emerald-200 rounded-t-sm transition-all relative overflow-hidden group-hover:bg-emerald-300 min-h-[4px]"
                                                                    style={{ height: `${Math.max(0, (item.value / maxVal) * 100)}%` }}
                                                                />
                                                            </div>
                                                            <span className="text-[10px] text-slate-400 mt-2 w-full text-center truncate rotate-0">{item.label}</span>
                                                        </div>
                                                    ));
                                                  })()}
                                                </div>
                                            )}
                                        </div>
                                        <div className="grid grid-cols-1 gap-6">
                       {/* Ranking Chart */}
                       <div className="bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden">
                          <div className="px-5 py-4 border-b border-slate-200 flex items-center justify-between">
                             <div className="flex items-center gap-2 font-medium text-slate-900">
                                {analysisTarget === 'all' ? <Award className="w-4 h-4 text-amber-500" /> : <PieChart className="w-4 h-4 text-blue-500" />}
                                {analysisData.rankingTitle}
                             </div>
                          </div>
                          <div className="p-4">
                             {/* Bar Chart for Ranking */}
                             <div className="space-y-3">
                                {analysisData.rankingData.map((item, idx) => (
                                    <div key={idx} className="flex items-center gap-3">
                                        <div className="w-32 text-xs text-slate-600 truncate text-right" title={item.name}>{item.name}</div>
                                        <div className="flex-1 h-3 bg-slate-100 rounded-full overflow-hidden relative">
                                            <div
                                                className={`h-full rounded-full ${analysisTarget === 'all' ? 'bg-amber-400' : 'bg-blue-400'}`}
                                                style={{ width: `${(item.value / analysisData.rankingData[0].value) * 100}%` }}
                                            />
                                        </div>
                                        <div className="w-24 text-xs font-medium text-slate-900 text-right">¥{item.value.toLocaleString()}</div>
                                    </div>
                                ))}
                             </div>
                          </div>
                       </div>
                    </div>
                  </>
                )}
             </div>
           )}

           {/* Logs Panel */}
           <div className={`bg-slate-900 flex-shrink-0 border-t border-slate-800 transition-all duration-300 flex flex-col ${showLogs ? "h-48" : "h-9"}`}>
              <div
                className="px-4 py-2 flex items-center justify-between cursor-pointer hover:bg-slate-800 flex-shrink-0"
                onClick={() => setShowLogs(!showLogs)}
              >
                <span className="text-xs text-slate-400 flex items-center gap-2 truncate pr-4">
                   {logs.length > 0 && (
                     logs[logs.length-1].level === 'error' ? <AlertCircle className="w-3 h-3 text-red-400" /> :
                     logs[logs.length-1].level === 'warning' ? <AlertCircle className="w-3 h-3 text-amber-400" /> :
                     <Check className="w-3 h-3 text-emerald-400" />
                   )}
                   {logs.length > 0 ? logs[logs.length-1].message : "就绪"}
                </span>
                <div className="flex items-center gap-3 flex-shrink-0">
                    <span className="text-xs text-slate-500 hover:text-slate-300">
                        {showLogs ? "收起" : "展开"}
                    </span>
                    <span className="text-xs text-slate-600">
                        {logs.length} 条日志
                    </span>
                    <ChevronUp className={`w-3 h-3 text-slate-500 transition-transform ${showLogs ? "rotate-180" : ""}`} />
                </div>
              </div>

              {showLogs && (
                <div className="px-4 pb-4 overflow-y-auto font-mono text-xs space-y-1 flex-1">
                  {logs.map((log, index) => (
                    <div
                      key={index}
                      className={`flex gap-3 ${
                        log.level === "error"
                          ? "text-red-400"
                          : log.level === "success"
                          ? "text-emerald-400"
                          : log.level === "warning"
                          ? "text-amber-400"
                          : "text-slate-300"
                      }`}
                    >
                      <span className="text-slate-600 flex-shrink-0 opacity-50">{log.timestamp}</span>
                      <span className="flex-shrink-0 mt-0.5">
                        {log.level === "error" && <AlertCircle className="w-3 h-3" />}
                        {log.level === "success" && <Check className="w-3 h-3" />}
                        {log.level === "warning" && <AlertCircle className="w-3 h-3" />}
                      </span>
                      <span>{log.message}</span>
                    </div>
                  ))}
                  <div ref={logEndRef} />
                </div>
              )}
           </div>
        </div>

      </div>

      {/* Success Modal */}
      {successModalOpen && (
        <div className="absolute inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4 animate-in fade-in duration-200">
          <div className="bg-white rounded-xl shadow-2xl max-w-md w-full p-6 transform transition-all animate-in zoom-in-95 duration-200">
            <div className="flex flex-col items-center text-center mb-6">
              <div className="w-12 h-12 bg-emerald-100 text-emerald-600 rounded-full flex items-center justify-center mb-4">
                <Check className="w-6 h-6" />
              </div>
              <h3 className="text-lg font-semibold text-slate-900">对账单生成成功</h3>
              <p className="text-xs text-slate-500 mt-2 break-all bg-slate-50 p-2 rounded border border-slate-100">
                {generatedFilePath}
              </p>
            </div>
            <div className="flex gap-3">
              <button
                onClick={() => setSuccessModalOpen(false)}
                className="flex-1 px-4 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50 font-medium transition-colors"
              >
                关闭
              </button>
              <button
                onClick={() => {
                  openFile(generatedFilePath);
                  setSuccessModalOpen(false);
                }}
                className="flex-1 px-4 py-2 bg-emerald-500 text-white rounded-lg hover:bg-emerald-600 font-medium transition-colors flex items-center justify-center gap-2"
              >
                <ExternalLink className="w-4 h-4" />
                打开文件
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;

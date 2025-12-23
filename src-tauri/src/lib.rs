mod data_processor;
mod excel_parser;
mod models;
mod statement_generator;

use data_processor::{
    group_by_customer_month, merge_delivery_data, scan_excel_files, validate_delivery_data,
};
use models::{AppConfig, ProcessResult, ScanResult};
use statement_generator::generate_statement;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;

const CONFIG_FILE: &str = "config.json";

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ana")
        .join(CONFIG_FILE)
}

/// 加载配置
#[tauri::command]
fn load_config() -> AppConfig {
    let config_path = get_config_path();
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

/// 保存配置
#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path();

    // 创建配置目录
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }

    // 序列化并保存
    let content =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, content).map_err(|e| format!("保存配置失败: {}", e))?;

    Ok(())
}

/// 扫描并验证数据
#[tauri::command]
async fn scan_and_validate(config: AppConfig) -> Result<ScanResult, String> {
    let raw_data_path = PathBuf::from(&config.raw_data_path);

    if !raw_data_path.exists() {
        return Ok(ScanResult {
            success: false,
            message: "原始数据目录不存在".to_string(),
            total_files: 0,
            valid_files: 0,
            errors: vec![],
            warnings: vec![],
            items: vec![],
        });
    }

    // 扫描文件
    let files =
        scan_excel_files(&raw_data_path).map_err(|e| format!("扫描文件失败: {}", e))?;

    if files.is_empty() {
        return Ok(ScanResult {
            success: true,
            message: "未找到 Excel 文件".to_string(),
            total_files: 0,
            valid_files: 0,
            errors: vec![],
            warnings: vec![],
            items: vec![],
        });
    }

    // 验证数据
    let (items, errors, warnings) = validate_delivery_data(&files);

    Ok(ScanResult {
        success: errors.is_empty(),
        message: if errors.is_empty() {
            if warnings.is_empty() {
                "数据验证通过".to_string()
            } else {
                format!("数据验证通过，但发现 {} 个警告", warnings.len())
            }
        } else {
            format!("发现 {} 个文件存在问题", errors.len())
        },
        total_files: files.len(),
        valid_files: files.len() - errors.len() - warnings.len(),
        errors,
        warnings,
        items,
    })
}

#[tauri::command]
async fn process_delivery_orders(
    app: tauri::AppHandle,
    config: AppConfig,
) -> Result<ProcessResult, String> {
    // 先保存配置
    save_config(config.clone())?;

    let raw_data_path = PathBuf::from(&config.raw_data_path);
    let output_path = PathBuf::from(&config.output_path);

    // 发送日志
    let _ = app.emit("log", "开始扫描 Excel 文件...");

    // 扫描 Excel 文件
    let files =
        scan_excel_files(&raw_data_path).map_err(|e| format!("扫描文件失败: {}", e))?;

    let _ = app.emit("log", format!("找到 {} 个 Excel 文件", files.len()));

    if files.is_empty() {
        return Err("未找到任何 Excel 文件".to_string());
    }

    // 合并数据
    let _ = app.emit("log", "正在合并送货单数据...");
    let all_items =
        merge_delivery_data(&files).map_err(|e| format!("合并数据失败: {}", e))?;

    let _ = app.emit("log", format!("共提取 {} 条数据记录", all_items.len()));

    if all_items.is_empty() {
        return Err("未提取到任何数据".to_string());
    }

    // 创建输出目录
    fs::create_dir_all(&output_path).map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 按客户和月份分组
    let grouped = group_by_customer_month(&all_items);
    let _ = app.emit(
        "log",
        format!("共有 {} 个客户月份组合", grouped.len()),
    );

    // 生成对账单
    let _ = app.emit("log", "开始生成对账单...");
    let mut generated_count = 0;
    let mut skipped_count = 0;

    for ((customer, year_month), items) in grouped.iter() {
        if customer.is_empty() {
            continue;
        }

        // 创建客户文件夹
        let customer_dir = output_path.join(customer);
        fs::create_dir_all(&customer_dir).map_err(|e| format!("创建客户文件夹失败: {}", e))?;

        // 生成文件名
        let statement_file =
            customer_dir.join(format!("statement_{}_{}.xlsx", customer, year_month));

        // 检查文件是否已存在
        if statement_file.exists() {
            let _ = app.emit(
                "log",
                format!("已存在，跳过: {} {}", customer, year_month),
            );
            skipped_count += 1;
            continue;
        }

        // 格式化年月
        let year_month_str = format_year_month(year_month);

        let _ = app.emit("log", format!("生成: {} {}", customer, year_month_str));

        // 生成对账单
        generate_statement(items, customer, &year_month_str, &statement_file, &config)
            .map_err(|e| format!("生成对账单失败: {}", e))?;

        generated_count += 1;
    }

    let _ = app.emit("log", "所有对账单生成完成！");
    let _ = app.emit("log", format!("新生成: {} 个对账单", generated_count));
    let _ = app.emit("log", format!("已跳过: {} 个对账单", skipped_count));

    Ok(ProcessResult {
        success: true,
        message: "处理完成".to_string(),
        generated_count,
        skipped_count,
        output_path: output_path.to_string_lossy().to_string(),
    })
}

fn format_year_month(year_month: &str) -> String {
    // 将 "2024-01" 格式化为 "2024年1月"
    let parts: Vec<&str> = year_month.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
            return format!("{}年{}月", year, month);
        }
    }
    year_month.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            process_delivery_orders,
            scan_and_validate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

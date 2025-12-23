mod models;
mod excel_parser;
mod data_processor;
mod statement_generator;

use models::{AppConfig, ProcessResult};
use data_processor::{scan_excel_files, merge_delivery_data, group_by_customer_month};
use statement_generator::generate_statement;
use std::path::PathBuf;
use std::fs;
use tauri::Emitter;

#[tauri::command]
fn get_default_config() -> AppConfig {
    AppConfig::default()
}

#[tauri::command]
fn save_config(_config: AppConfig) -> Result<(), String> {
    // 这里可以将配置保存到文件
    // 暂时只返回成功
    Ok(())
}

#[tauri::command]
async fn process_delivery_orders(
    app: tauri::AppHandle,
    config: AppConfig,
) -> Result<ProcessResult, String> {
    let raw_data_path = PathBuf::from(&config.raw_data_path);
    let output_path = PathBuf::from(&config.output_path);

    // 发送日志
    let _ = app.emit("log", "开始扫描 Excel 文件...");

    // 扫描 Excel 文件
    let files = scan_excel_files(&raw_data_path)
        .map_err(|e| format!("扫描文件失败: {}", e))?;

    let _ = app.emit("log", format!("找到 {} 个 Excel 文件", files.len()));

    if files.is_empty() {
        return Err("未找到任何 Excel 文件".to_string());
    }

    // 合并数据
    let _ = app.emit("log", "正在合并送货单数据...");
    let all_items = merge_delivery_data(&files)
        .map_err(|e| format!("合并数据失败: {}", e))?;

    let _ = app.emit("log", format!("共提取 {} 条数据记录", all_items.len()));

    if all_items.is_empty() {
        return Err("未提取到任何数据".to_string());
    }

    // 创建输出目录
    fs::create_dir_all(&output_path)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 保存合并后的数据
    let _merged_file = output_path.join("merged_delivery_orders.xlsx");
    let _ = app.emit("log", "正在保存合并数据...");

    // 这里应该保存合并的 Excel 文件，暂时跳过
    // save_merged_excel(&all_items, &merged_file)?;

    // 按客户和月份分组
    let grouped = group_by_customer_month(&all_items);
    let _ = app.emit("log", format!("共有 {} 个客户月份组合", grouped.len()));

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
        fs::create_dir_all(&customer_dir)
            .map_err(|e| format!("创建客户文件夹失败: {}", e))?;

        // 生成文件名
        let statement_file = customer_dir.join(format!("statement_{}_{}.xlsx", customer, year_month));

        // 检查文件是否已存在
        if statement_file.exists() {
            let _ = app.emit("log", format!("已存在，跳过: {} {}", customer, year_month));
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
            get_default_config,
            save_config,
            process_delivery_orders
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

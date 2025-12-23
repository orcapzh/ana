use crate::excel_parser::extract_delivery_data;
use crate::models::{DeliveryItem, SummaryItem};
use anyhow::Result;
use chrono::Datelike;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 扫描目录中的所有 Excel 文件
pub fn scan_excel_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "xls" || ext_str == "xlsx" {
                // 跳过临时文件
                if !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("~$")
                {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

/// 合并所有送货单数据
pub fn merge_delivery_data(files: &[PathBuf]) -> Result<Vec<DeliveryItem>> {
    let mut all_items = Vec::new();

    for file in files {
        match extract_delivery_data(file) {
            Ok(items) => {
                all_items.extend(items);
            }
            Err(e) => {
                eprintln!("处理文件 {:?} 时出错: {}", file, e);
            }
        }
    }

    Ok(all_items)
}

/// 生成汇总数据
pub fn generate_summary(items: &[DeliveryItem]) -> Vec<SummaryItem> {
    let mut summary_map: HashMap<(String, String, String), SummaryItem> = HashMap::new();

    for item in items {
        let key = (
            item.product_name.clone(),
            item.spec.clone(),
            item.unit.clone(),
        );

        summary_map
            .entry(key.clone())
            .and_modify(|summary| {
                summary.quantity += item.quantity;
                summary.amount += item.amount;
                // 添加客户到列表（去重）
                if !item.customer.is_empty() {
                    let customers: Vec<String> = summary
                        .customers
                        .split(", ")
                        .map(|s| s.to_string())
                        .collect();
                    if !customers.contains(&item.customer) {
                        if !summary.customers.is_empty() {
                            summary.customers.push_str(", ");
                        }
                        summary.customers.push_str(&item.customer);
                    }
                }
            })
            .or_insert_with(|| SummaryItem {
                product_name: key.0.clone(),
                spec: key.1.clone(),
                unit: key.2.clone(),
                quantity: item.quantity,
                average_price: 0.0,
                amount: item.amount,
                customers: item.customer.clone(),
            });
    }

    // 计算平均单价
    let mut summary_vec: Vec<SummaryItem> = summary_map.into_values().collect();
    for item in &mut summary_vec {
        if item.quantity > 0.0 {
            item.average_price = item.amount / item.quantity;
            item.average_price = (item.average_price * 100.0).round() / 100.0;
        }
    }

    // 按金额降序排列
    summary_vec.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap());

    summary_vec
}

/// 按客户和月份分组
pub fn group_by_customer_month(items: &[DeliveryItem]) -> HashMap<(String, String), Vec<DeliveryItem>> {
    let mut groups: HashMap<(String, String), Vec<DeliveryItem>> = HashMap::new();

    for item in items {
        // 提取年月
        let year_month = extract_year_month(&item.date);

        let key = (item.customer.clone(), year_month);
        groups.entry(key).or_insert_with(Vec::new).push(item.clone());
    }

    groups
}

/// 从日期字符串中提取年月
fn extract_year_month(date_str: &str) -> String {
    // 尝试多种日期格式
    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y年%m月%d日",
        "%Y-%m-%d %H:%M:%S",
    ];

    for format in &formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format) {
            return format!("{}-{:02}", date.year(), date.month());
        }
    }

    // 如果无法解析，尝试从字符串中提取年月
    if let Some(pos) = date_str.find('-') {
        if let Some(second_pos) = date_str[pos + 1..].find('-') {
            return date_str[0..pos + 1 + second_pos].to_string();
        }
    }

    "未知".to_string()
}

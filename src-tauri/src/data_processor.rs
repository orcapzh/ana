use crate::excel_parser::extract_delivery_data;
use crate::models::{DeliveryItem, FileValidationError, SummaryItem};
use anyhow::Result;
use chrono::Datelike;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 扫描目录中的所有 Excel 文件，并根据一级子目录确定客户类型
/// 目录结构: Root -> Type (现金客户/月结客户) -> ... -> Files
pub fn scan_excel_files(dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    // 读取根目录下的第一级子目录作为客户类型
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // 获取目录名作为类型 (e.g. "现金客户")
            let type_name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // 递归扫描该类型目录下的所有 Excel 文件
            for walk_entry in WalkDir::new(&path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let file_path = walk_entry.path();
                if let Some(ext) = file_path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "xls" || ext_str == "xlsx" {
                        // 跳过临时文件
                        if !file_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .starts_with("~$")
                        {
                            files.push((file_path.to_path_buf(), type_name.clone()));
                        }
                    }
                }
            }
        } else {
             // 如果根目录下直接有文件，归类为 "未分类" 或 "默认"
             if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "xls" || ext_str == "xlsx" {
                    if !path.file_name().unwrap_or_default().to_string_lossy().starts_with("~$") {
                        files.push((path.to_path_buf(), "默认".to_string()));
                    }
                }
            }
        }
    }

    Ok(files)
}

/// 合并所有送货单数据
pub fn merge_delivery_data(files: &[(PathBuf, String)]) -> Result<Vec<DeliveryItem>> {
    let mut all_items = Vec::new();

    for (file, customer_type) in files {
        match extract_delivery_data(file, customer_type) {
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

/// 验证并合并送货单数据
pub fn validate_delivery_data(
    files: &[(PathBuf, String)],
) -> (
    Vec<DeliveryItem>,
    Vec<FileValidationError>,
    Vec<FileValidationError>,
) {
    let mut all_items = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    // 记录 (客户, 单号) 及其来源文件，用于同客户内的单号查重: (customer, order_no) -> file_path
    let mut order_no_map: HashMap<(String, String), String> = HashMap::new();

    for (file, customer_type) in files {
        match extract_delivery_data(file, customer_type) {
            Ok(items) => {
                if items.is_empty() {
                    warnings.push(FileValidationError {
                        file: file.to_string_lossy().to_string(),
                        error: "该文件未包含有效数据或格式不匹配".to_string(),
                    });
                } else {
                    let mut file_has_error = false;
                    
                    // 1. 尝试从文件名提取日期
                    let file_name = file.file_name().unwrap_or_default().to_string_lossy();
                    let file_date = extract_date_from_filename(&file_name);
                    
                    for item in &items {
                        // 验证日期格式
                        if let Err(e) = validate_date_str(&item.date) {
                            errors.push(FileValidationError {
                                file: file.to_string_lossy().to_string(),
                                error: format!("日期错误 '{}': {}", item.date, e),
                            });
                            file_has_error = true;
                        } else {
                            // 2. 验证文件名日期与内容日期是否一致
                            if let Some(f_date) = file_date {
                                if let Ok(c_date) = parse_date(&item.date) {
                                    if f_date != c_date {
                                         warnings.push(FileValidationError {
                                            file: file.to_string_lossy().to_string(),
                                            error: format!("日期不一致: 文件名日期 ({}) 与内容日期 ({}) 不同", f_date, c_date),
                                        });
                                    }
                                }
                            }
                        }

                        // 3. 验证送货单号是否重复 (仅针对同一个客户)
                        if !item.delivery_order_no.is_empty() {
                            let order_key = (item.customer.clone(), item.delivery_order_no.clone());
                            if let Some(existing_file) = order_no_map.get(&order_key) {
                                let current_file = file.to_string_lossy().to_string();
                                if *existing_file != current_file {
                                    warnings.push(FileValidationError {
                                        file: current_file.clone(),
                                        error: format!("送货单号重复: 客户 '{}' 的单号 '{}' 已在文件 '{}' 中存在", 
                                            item.customer, order_key.1, existing_file.split(|c| c == '/' || c == '\\').last().unwrap_or(existing_file)),
                                    });
                                }
                            } else {
                                order_no_map.insert(order_key, file.to_string_lossy().to_string());
                            }
                        }
                    }

                    if !file_has_error {
                        all_items.extend(items);
                    }
                }
            }
            Err(e) => {
                errors.push(FileValidationError {
                    file: file.to_string_lossy().to_string(),
                    error: format!("解析失败: {}", e),
                });
            }
        }
    }
    
    // 去重 warnings (因为循环中可能多次添加相同的警告)
    warnings.sort_by(|a, b| a.file.cmp(&b.file).then(a.error.cmp(&b.error)));
    warnings.dedup_by(|a, b| a.file == b.file && a.error == b.error);

    (all_items, errors, warnings)
}

fn extract_date_from_filename(filename: &str) -> Option<chrono::NaiveDate> {
    use regex::Regex;
    // 匹配 YYYY-MM-DD, YYYY.MM.DD, YYYYMMDD 等
    // 简单起见，匹配 202x-xx-xx 或 202xxx-xx 等常见格式
    // 优先匹配标准格式
    if let Ok(re) = Regex::new(r"(\d{4})[-.](\d{1,2})[-.](\d{1,2})") {
        if let Some(caps) = re.captures(filename) {
            let y = caps[1].parse::<i32>().ok()?;
            let m = caps[2].parse::<u32>().ok()?;
            let d = caps[3].parse::<u32>().ok()?;
            return chrono::NaiveDate::from_ymd_opt(y, m, d);
        }
    }
    None
}

fn parse_date(date_str: &str) -> Result<chrono::NaiveDate, String> {
    let formats = ["%Y-%m-%d", "%Y/%m/%d", "%Y年%m月%d日"];
    for format in &formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format) {
            return Ok(date);
        }
    }
    Err("Invalid date".to_string())
}

fn validate_date_str(date_str: &str) -> Result<(), String> {
    // 尝试解析日期
    let formats = ["%Y-%m-%d", "%Y/%m/%d", "%Y年%m月%d日"];

    for format in &formats {
        if let Ok(_) = chrono::NaiveDate::parse_from_str(date_str, format) {
            return Ok(());
        }
    }

    Err("无法识别的日期格式或日期无效".to_string())
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

use crate::models::DeliveryItem;
use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

/// 从 Excel 文件中提取送货单数据
pub fn extract_delivery_data(file_path: &Path, customer_type: &str) -> Result<Vec<DeliveryItem>> {
    let mut workbook = open_workbook_auto(file_path)
        .with_context(|| format!("无法打开文件: {:?}", file_path))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .context("工作簿没有工作表")?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .context("无法读取工作表")?;

    let mut items = Vec::new();

    let mut customer_name = String::new();
    let mut date = String::new();
    let mut delivery_order_no = String::new();
    let mut header_order_no = String::new();

    // 扫描前 10 行以提取 客户、日期、单号、全局订单号
    for row_idx in 0..10 {
        if let Some(row) = range.rows().nth(row_idx) {
            for (col_idx, cell) in row.iter().enumerate() {
                let cell_str = cell.to_string().trim().to_string();
                let cell_lower = cell_str.to_lowercase();

                // 1. 提取客户名称
                if (cell_str.contains("客户") || cell_str.contains("单位")) && customer_name.is_empty() {
                    let parts: Vec<&str> = cell_str.split(|c| c == ':' || c == '：').collect();
                    if parts.len() > 1 && !parts[1].trim().is_empty() {
                        customer_name = parts[1].trim().to_string();
                    } else if let Some(next_cell) = row.get(col_idx + 1) {
                        let val = next_cell.to_string().trim().to_string();
                        if !val.is_empty() {
                            customer_name = val;
                        }
                    }
                }

                // 2. 提取日期
                if cell_str.contains("日期") && date.is_empty() {
                    let parts: Vec<&str> = cell_str.split(|c| c == ':' || c == '：').collect();
                    if parts.len() > 1 && !parts[1].trim().is_empty() {
                        date = normalize_date(parts[1].trim());
                    } else if let Some(next_cell) = row.get(col_idx + 1) {
                        date = excel_date_to_string(next_cell);
                    }
                }

                // 3. 提取送货单号 (No)
                if (cell_lower.contains("no") || cell_lower.contains("单号")) && delivery_order_no.is_empty() {
                    let parts: Vec<&str> = cell_str.split(|c| c == ':' || c == '：' || c == '.' || c == ' ').collect();
                    for (p_idx, part) in parts.iter().enumerate() {
                        let p_trimmed = part.trim();
                        let p_lower = p_trimmed.to_lowercase();
                        if (p_lower == "no" || p_lower == "单号") && p_idx + 1 < parts.len() {
                            let val = parts[p_idx + 1].trim();
                            if !val.is_empty() {
                                delivery_order_no = val.to_string();
                                break;
                            }
                        }
                    }
                    if delivery_order_no.is_empty() && cell_lower.starts_with("no") {
                        let val = &cell_str[2..].trim();
                        if !val.is_empty() {
                            delivery_order_no = val.to_string();
                        }
                    }
                    if delivery_order_no.is_empty() {
                        if let Some(next_cell) = row.get(col_idx + 1) {
                            let next_str = next_cell.to_string().trim().to_string();
                            if !next_str.is_empty() {
                                delivery_order_no = next_str;
                            }
                        }
                    }
                }

                // 4. 提取全局订单号 (例如在备注或标题区域)
                if cell_str.contains("订单号") && header_order_no.is_empty() {
                    let parts: Vec<&str> = cell_str.split(|c| c == ':' || c == '：').collect();
                    if parts.len() > 1 && !parts[1].trim().is_empty() {
                        header_order_no = parts[1].trim().to_string();
                    } else if let Some(next_cell) = row.get(col_idx + 1) {
                        let val = next_cell.to_string().trim().to_string();
                        if !val.is_empty() {
                            header_order_no = val;
                        }
                    }
                }
            }
        }
    }

    // 数据起始行及列识别
    let mut data_start_row = 8;
    let mut col_map = std::collections::HashMap::new();

    for row_idx in 0..15 {
        if let Some(row) = range.rows().nth(row_idx) {
            let mut found_header = false;
            for (col_idx, cell) in row.iter().enumerate() {
                let s = cell.to_string();
                if s.contains("货名") || s.contains("货品名称") || s.contains("Description") {
                    col_map.insert("product", col_idx);
                    found_header = true;
                } else if s.contains("规格") {
                    col_map.insert("spec", col_idx);
                } else if s.contains("数量") || s.contains("Quantity") {
                    col_map.insert("quantity", col_idx);
                } else if s.contains("单位") || s.contains("unit") {
                    col_map.insert("unit", col_idx);
                } else if s.contains("单价") || s.contains("Unit Price") || s.contains("价格") || s.contains("Price") {
                    col_map.insert("price", col_idx);
                } else if s.contains("金额") || s.contains("Amount") || s.contains("总价") {
                    col_map.insert("amount", col_idx);
                } else if s.contains("订单号") || s.contains("PO") {
                    col_map.insert("order_no", col_idx);
                }
            }
            if found_header {
                data_start_row = row_idx + 1;
                break;
            }
        }
    }

    let idx_product = *col_map.get("product").unwrap_or(&0);
    let idx_spec = *col_map.get("spec").unwrap_or(&2);
    let idx_quantity = *col_map.get("quantity").unwrap_or(&4);
    let idx_unit = *col_map.get("unit").unwrap_or(&5);
    let idx_price = col_map.get("price").cloned();
    let idx_amount = col_map.get("amount").cloned();
    let idx_order_no = col_map.get("order_no").cloned();

    for (idx, row) in range.rows().enumerate() {
        if idx < data_start_row {
            continue;
        }

        // 检查是否到达合计行
        let first_cell = row.get(0).map(|c| c.to_string()).unwrap_or_default();
        if first_cell.contains("合计") || first_cell.contains("送货单位") {
            break;
        }

        // 提取货名
        let product_name = row
            .get(idx_product)
            .map(|c| c.to_string().replace('\n', " ").replace('"', "").trim().to_string())
            .filter(|s| !s.is_empty());

        // 跳过空行
        if product_name.is_none() {
            continue;
        }

        // 提取规格
        let spec = row
            .get(idx_spec)
            .map(|c| c.to_string().trim().to_string())
            .unwrap_or_default();

        // 提取数量
        let quantity = row.get(idx_quantity).and_then(|c| extract_number(c));

        // 跳过没有数量的行
        if quantity.is_none() {
            continue;
        }

        // 提取单位
        let unit = row
            .get(idx_unit)
            .map(|c| c.to_string().trim().to_string())
            .unwrap_or_default();

        // 提取单价
        let unit_price = idx_price.and_then(|i| row.get(i)).and_then(|c| extract_number(c)).unwrap_or(0.0);

        // 提取金额
        let amount = idx_amount.and_then(|i| row.get(i)).and_then(|c| extract_number(c)).unwrap_or(0.0);

        // 尝试从当前行的所有单元格中提取“订单号：xxxx” (处理埋在备注里的情况)
        for cell in row.iter() {
            let s = cell.to_string();
            if s.contains("订单号") {
                let parts: Vec<&str> = s.split(|c| c == ':' || c == '：').collect();
                if parts.len() > 1 && !parts[1].trim().is_empty() {
                    let extracted = parts[1].trim().to_string();
                    if header_order_no.is_empty() || header_order_no == extracted {
                        header_order_no = extracted;
                    }
                }
            }
        }

        // 提取订单号 (优先用列数据，没有则用全局/行内识别到的)
        let row_order_no = idx_order_no
            .and_then(|i| row.get(i))
            .map(|c| c.to_string().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| header_order_no.clone());

        items.push(DeliveryItem {
            product_name: product_name.unwrap(),
            spec,
            quantity: quantity.unwrap(),
            unit,
            unit_price,
            amount,
            customer: customer_name.clone(),
            date: date.clone(),
            delivery_order_no: delivery_order_no.clone(),
            order_no: row_order_no,
            source_file: file_path.to_string_lossy().to_string(),
            customer_type: customer_type.to_string(),
        });
    }

    Ok(items)
}

/// 从单元格提取数字
fn extract_number(cell: &Data) -> Option<f64> {
    match cell {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => {
            let s = s.trim();
            if s.is_empty() { return None; }
            if let Ok(f) = s.parse::<f64>() {
                return Some(f);
            }
            // 尝试提取开头的数字部分 (处理类似 "160*1000米" 的情况)
            let mut num_str = String::new();
            for c in s.chars() {
                if c.is_numeric() || c == '.' || c == '-' {
                    num_str.push(c);
                } else {
                    break;
                }
            }
            num_str.parse::<f64>().ok()
        }
        _ => None,
    }
}

/// 将 Excel 日期单元格转换为日期字符串
fn excel_date_to_string(cell: &Data) -> String {
    match cell {
        Data::DateTime(dt) => excel_serial_to_date(dt.as_f64() as i64),
        Data::Float(f) => excel_serial_to_date(*f as i64),
        Data::Int(i) => excel_serial_to_date(*i),
        Data::String(s) => normalize_date(s.trim()),
        _ => String::new(),
    }
}

/// 标准化日期字符串为 YYYY-MM-DD
fn normalize_date(date_str: &str) -> String {
    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y年%m月%d日",
        "%d/%m/%Y",
        "%m/%d/%Y",
    ];

    for format in &formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format) {
            return date.format("%Y-%m-%d").to_string();
        }
    }

    date_str.to_string()
}

/// Excel 日期序列号转日期
fn excel_serial_to_date(serial: i64) -> String {
    use chrono::{Duration, NaiveDate};
    // Excel 日期从 1899-12-30 开始（因为 Excel 的 1900 年闰年 bug）
    let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let date = base + Duration::days(serial);
    date.format("%Y-%m-%d").to_string()
}

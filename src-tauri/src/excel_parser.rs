use crate::models::DeliveryItem;
use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

/// 从 Excel 文件中提取送货单数据
pub fn extract_delivery_data(file_path: &Path) -> Result<Vec<DeliveryItem>> {
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

    // 提取客户名称（第5行，索引4，列1）
    let customer_name = range
        .rows()
        .nth(4)
        .and_then(|row| row.get(1))
        .map(|c| c.to_string().trim().to_string())
        .unwrap_or_default();

    // 提取日期（第5行，索引4，列7）- Excel日期序列号
    let date = range
        .rows()
        .nth(4)
        .and_then(|row| row.get(7))
        .map(|c| excel_date_to_string(c))
        .unwrap_or_default();

    // 数据从第9行开始（索引8）
    for (idx, row) in range.rows().enumerate() {
        if idx < 8 {
            continue;
        }

        // 检查是否到达合计行
        let first_cell = row.get(0).map(|c| c.to_string()).unwrap_or_default();
        if first_cell.contains("合计") {
            break;
        }

        // 提取货名（列0）
        let product_name = row
            .get(0)
            .map(|c| c.to_string().replace('\n', " ").replace('"', "").trim().to_string())
            .filter(|s| !s.is_empty());

        // 跳过空行
        if product_name.is_none() {
            continue;
        }

        // 提取规格（列2）
        let spec = row
            .get(2)
            .map(|c| c.to_string().trim().to_string())
            .unwrap_or_default();

        // 提取数量（列4）
        let quantity = row.get(4).and_then(|c| extract_number(c));

        // 跳过没有数量的行
        if quantity.is_none() {
            continue;
        }

        // 提取单位（列5）
        let unit = row
            .get(5)
            .map(|c| c.to_string().trim().to_string())
            .unwrap_or_default();

        // 提取单价（列6）
        let unit_price = row.get(6).and_then(|c| extract_number(c)).unwrap_or(0.0);

        // 提取金额（列7）
        let amount = row.get(7).and_then(|c| extract_number(c)).unwrap_or(0.0);

        items.push(DeliveryItem {
            product_name: product_name.unwrap(),
            spec,
            quantity: quantity.unwrap(),
            unit,
            unit_price,
            amount,
            customer: customer_name.clone(),
            date: date.clone(),
            source_file: file_path.to_string_lossy().to_string(),
        });
    }

    Ok(items)
}

/// 从单元格提取数字
fn extract_number(cell: &Data) -> Option<f64> {
    match cell {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// 将 Excel 日期单元格转换为日期字符串
fn excel_date_to_string(cell: &Data) -> String {
    match cell {
        Data::DateTime(dt) => excel_serial_to_date(dt.as_f64() as i64),
        Data::Float(f) => excel_serial_to_date(*f as i64),
        Data::Int(i) => excel_serial_to_date(*i),
        Data::String(s) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Excel 日期序列号转日期
fn excel_serial_to_date(serial: i64) -> String {
    use chrono::{Duration, NaiveDate};
    // Excel 日期从 1899-12-30 开始（因为 Excel 的 1900 年闰年 bug）
    let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let date = base + Duration::days(serial);
    date.format("%Y-%m-%d").to_string()
}

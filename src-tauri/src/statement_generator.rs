use crate::models::{AppConfig, DeliveryItem};
use anyhow::Result;
use rust_xlsxwriter::*;
use std::path::Path;

/// 生成对账单
pub fn generate_statement(
    items: &[DeliveryItem],
    customer_name: &str,
    year_month: &str,
    output_file: &Path,
    config: &AppConfig,
) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 设置列宽
    worksheet.set_column_width(0, 12)?;
    worksheet.set_column_width(1, 20)?;
    worksheet.set_column_width(2, 8)?;
    worksheet.set_column_width(3, 10)?;
    worksheet.set_column_width(4, 10)?;
    worksheet.set_column_width(5, 12)?;
    worksheet.set_column_width(6, 12)?;

    // 标题格式
    let title_format = Format::new()
        .set_font_size(18)
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    let subtitle_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    let header_format = Format::new()
        .set_font_size(11)
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color(Color::RGB(0xD3D3D3))
        .set_border(FormatBorder::Thin);

    let cell_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);

    let wrap_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap()
        .set_border(FormatBorder::Thin);

    // 标题行（第1行）
    worksheet.merge_range(0, 0, 0, 6, &config.company_name, &title_format)?;
    worksheet.set_row_height(0, 30)?;

    // 地址行（第2行）
    let address_text = format!("地址：{}", config.address);
    worksheet.merge_range(1, 0, 1, 6, &address_text, &subtitle_format)?;

    // 联系方式行（第3行）
    let contact_text = format!("电话：{}    传真：{}", config.phone, config.fax);
    worksheet.merge_range(2, 0, 2, 6, &contact_text, &subtitle_format)?;

    // 客户和日期信息（第4行）
    let customer_text = format!("客户：{}", customer_name);
    worksheet.merge_range(3, 0, 3, 1, &customer_text, &Format::new())?;

    let month_text = format!("{}对账单", year_month);
    worksheet.merge_range(
        3,
        2,
        3,
        4,
        &month_text,
        &Format::new().set_align(FormatAlign::Center),
    )?;

    // 表头（第5行）
    let headers = ["送货日期", "品名规格", "单位", "数量", "单价", "金额", "备注"];
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_with_format(4, col as u16, *header, &header_format)?;
    }

    // 数据行（从第6行开始）
    let mut sorted_items: Vec<&DeliveryItem> = items.iter().collect();
    sorted_items.sort_by(|a, b| a.date.cmp(&b.date));

    let mut total_amount = 0.0;
    for (idx, item) in sorted_items.iter().enumerate() {
        let row = (idx + 5) as u32;

        // 日期
        let date_str = format_date(&item.date);
        worksheet.write_with_format(row, 0, &date_str, &cell_format)?;

        // 品名规格
        let product_spec = format!("{} {}", item.product_name, item.spec);
        worksheet.write_with_format(row, 1, &product_spec, &wrap_format)?;

        // 单位
        worksheet.write_with_format(row, 2, &item.unit, &cell_format)?;

        // 数量
        worksheet.write_with_format(row, 3, item.quantity, &cell_format)?;

        // 单价
        worksheet.write_with_format(row, 4, item.unit_price, &cell_format)?;

        // 金额
        worksheet.write_with_format(row, 5, item.amount, &cell_format)?;

        // 备注
        worksheet.write_with_format(row, 6, "", &cell_format)?;

        total_amount += item.amount;
    }

    // 合计行
    let summary_row = (sorted_items.len() + 7) as u32;

    // 中文大写金额
    let chinese_amount = amount_to_chinese(total_amount);
    let summary_text = format!("合计人民币大写：{}", chinese_amount);
    worksheet.merge_range(
        summary_row,
        0,
        summary_row,
        2,
        &summary_text,
        &Format::new().set_font_size(11),
    )?;

    // 小写金额
    let amount_text = format!("人民币小写：{:.2}元", total_amount);
    worksheet.merge_range(
        summary_row,
        3,
        summary_row,
        6,
        &amount_text,
        &Format::new()
            .set_font_size(11)
            .set_align(FormatAlign::Right),
    )?;

    // 保存文件
    workbook.save(output_file)?;

    Ok(())
}

/// 格式化日期
fn format_date(date_str: &str) -> String {
    // 尝试解析日期并格式化
    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y年%m月%d日",
        "%Y-%m-%d %H:%M:%S",
    ];

    for format in &formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format) {
            return date.format("%Y-%m-%d").to_string();
        }
    }

    // 如果无法解析，返回原始字符串
    date_str
        .split('T')
        .next()
        .unwrap_or(date_str)
        .to_string()
}

/// 将金额转换为中文大写
fn amount_to_chinese(amount: f64) -> String {
    let chinese_numbers = [
        "零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖",
    ];
    let chinese_units = ["", "拾", "佰", "仟", "万", "拾", "佰", "仟", "亿"];

    let amount_str = format!("{:.2}", amount);
    let parts: Vec<&str> = amount_str.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1).unwrap_or(&"00");

    // 转换整数部分
    let mut result = String::new();
    let chars: Vec<char> = integer_part.chars().rev().collect();

    for (i, ch) in chars.iter().enumerate() {
        let digit = ch.to_digit(10).unwrap_or(0) as usize;
        if digit != 0 {
            result = format!(
                "{}{}{}",
                chinese_numbers[digit], chinese_units[i], result
            );
        } else if !result.is_empty() && !result.starts_with("零") {
            result = format!("零{}", result);
        }
    }

    // 清理多余的零
    while result.contains("零零") {
        result = result.replace("零零", "零");
    }
    if result.ends_with("零") {
        result.pop();
    }
    if result.is_empty() {
        result = "零".to_string();
    }

    result.push_str("元");

    // 处理角分
    let jiao = decimal_part
        .chars()
        .next()
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as usize;
    let fen = decimal_part
        .chars()
        .nth(1)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as usize;

    if jiao == 0 && fen == 0 {
        result.push_str("整");
    } else {
        if jiao != 0 {
            result.push_str(chinese_numbers[jiao]);
            result.push_str("角");
        }
        if fen != 0 {
            result.push_str(chinese_numbers[fen]);
            result.push_str("分");
        }
    }

    result
}

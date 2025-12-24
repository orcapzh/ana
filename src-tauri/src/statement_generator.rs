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

    // 检查是否包含订单号
    let has_order_no = items.iter().any(|i| !i.order_no.is_empty());
    // 总列数索引 (例如：日期、送货单号、[订单号]、品名规格、单位、数量、单价、金额、备注)
    // 有订单号共9列 (0-8)，无订单号共8列 (0-7)
    let total_cols = if has_order_no { 8 } else { 7 };

    // 设置列宽
    worksheet.set_column_width(0, 12)?; // 日期
    worksheet.set_column_width(1, 15)?; // 送货单号
    
    let mut current_col = 2;
    if has_order_no {
        worksheet.set_column_width(current_col, 15)?; // 订单号
        current_col += 1;
    }
    
    // 如果没有订单号，给品名规格更多空间
    let product_width = if has_order_no { 20 } else { 35 };
    worksheet.set_column_width(current_col, product_width)?; // 品名规格
    current_col += 1;
    
    worksheet.set_column_width(current_col, 8)?;  // 单位
    current_col += 1;
    let qty_col_idx = current_col;
    worksheet.set_column_width(current_col, 10)?; // 数量
    current_col += 1;
    let price_col_idx = current_col;
    worksheet.set_column_width(current_col, 10)?; // 单价
    current_col += 1;
    let amount_col_idx = current_col;
    worksheet.set_column_width(current_col, 12)?; // 金额
    current_col += 1;
    worksheet.set_column_width(current_col, 12)?; // 备注

    // 格式定义
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

    let amount_cell_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin)
        .set_num_format("¥#,##0.00");

    let wrap_format = Format::new()
        .set_font_size(10)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap()
        .set_border(FormatBorder::Thin);

    // 标题行
    worksheet.merge_range(0, 0, 0, total_cols as u16, &config.company_name, &title_format)?;
    worksheet.set_row_height(0, 30)?;

    // 地址行
    let address_text = format!("地址：{}", config.address);
    worksheet.merge_range(1, 0, 1, total_cols as u16, &address_text, &subtitle_format)?;

    // 联系方式行
    let contact_text = format!("电话：{}    传真：{}", config.phone, config.fax);
    worksheet.merge_range(2, 0, 2, total_cols as u16, &contact_text, &subtitle_format)?;

    // 客户和日期信息
    let customer_text = format!("客户：{}", customer_name);
    worksheet.merge_range(3, 0, 3, 2, &customer_text, &Format::new())?;

    let month_text = format!("{}对账单", year_month);
    worksheet.merge_range(
        3,
        3,
        3,
        5,
        &month_text,
        &Format::new().set_align(FormatAlign::Center),
    )?;

    // 表头
    let mut headers = vec!["送货日期", "送货单号"];
    if has_order_no {
        headers.push("订单号");
    }
    headers.extend(["品名规格", "单位", "数量", "单价", "金额", "备注"]);
    
    for (col, header) in headers.iter().enumerate() {
        worksheet.write_with_format(4, col as u16, *header, &header_format)?;
    }

    // 数据行
    let mut sorted_items: Vec<&DeliveryItem> = items.iter().collect();
    sorted_items.sort_by(|a, b| a.date.cmp(&b.date));

    let start_data_row = 6; // Excel 1-based index for row 6 (index 5)
    let mut last_data_row = start_data_row;

    let mut total_amount = 0.0;
    for (idx, item) in sorted_items.iter().enumerate() {
        let row = (idx + 5) as u32;
        let excel_row = row + 1;
        last_data_row = excel_row;
        let mut col = 0;

        // 日期
        worksheet.write_with_format(row, col, &format_date(&item.date), &cell_format)?;
        col += 1;

        // 送货单号
        worksheet.write_with_format(row, col, &item.delivery_order_no, &cell_format)?;
        col += 1;

        // 订单号 (可选)
        if has_order_no {
            worksheet.write_with_format(row, col, &item.order_no, &cell_format)?;
            col += 1;
        }

        // 品名规格
        let product_spec = format!("{} {}", item.product_name, item.spec);
        worksheet.write_with_format(row, col, &product_spec, &wrap_format)?;
        col += 1;

        // 单位
        worksheet.write_with_format(row, col, &item.unit, &cell_format)?;
        col += 1;

        // 数量
        worksheet.write_with_format(row, col, item.quantity, &cell_format)?;
        col += 1;

        // 单价
        worksheet.write_with_format(row, col, item.unit_price, &cell_format)?;
        col += 1;

        // 金额 (公式: 数量 * 单价)
        let qty_cell = format!("{}{}", utility::column_number_to_name(qty_col_idx as u16), excel_row);
        let price_cell = format!("{}{}", utility::column_number_to_name(price_col_idx as u16), excel_row);
        let amount_formula = format!("={}*{}", qty_cell, price_cell);
        worksheet.write_formula_with_format(row, col, amount_formula.as_str(), &amount_cell_format)?;
        col += 1;

        // 备注
        worksheet.write_with_format(row, col, "", &cell_format)?;

        total_amount += item.amount;
    }

    // 合计行
    let summary_row = (sorted_items.len() + 7) as u32;
    let amount_col_name = utility::column_number_to_name(amount_col_idx as u16);
    
    // 预计算初始大写文字 (用于 Numbers 等不支持公式的环境)
    let initial_chinese = amount_to_chinese(total_amount);
    
    // 构造大写转换公式 (针对 Excel/WPS 环境)
    let sum_ref = format!("SUM({}{}:{}{})", amount_col_name, start_data_row, amount_col_name, last_data_row);
    let caps_formula = format!(
        "=\"合计人民币大写：\" & IF({0}=0,\"零元整\",IF({0}<0,\"负\",\"\") & SUBSTITUTE(SUBSTITUTE(SUBSTITUTE(TEXT(INT(ABS({0})),\"[DBNum2]0元\") & TEXT(MOD(INT(ABS({0})*10),10),\"[DBNum2]0角\") & TEXT(MOD(INT(ABS({0})*100),10),\"[DBNum2]0分\"),\"零角零分\",\"整\"),\"零分\",\"整\"),\"零角\",\"零\"))",
        sum_ref
    );

    // 中文大写合计
    worksheet.merge_range(
        summary_row,
        0,
        summary_row,
        3,
        "",
        &Format::new().set_font_size(11),
    )?;
    
    // 写入公式。注意：由于 Numbers 不支持此公式，它可能会显示错误或 0。
    // 但在生成时，我们已经提供了初始金额。
    worksheet.write_formula_with_format(
        summary_row,
        0,
        caps_formula.as_str(),
        &Format::new().set_font_size(11)
    )?;
    
    // 如果是第一次打开，公式可能还没运行，有些软件会显示我们写入的“默认值”
    // 这里我们强制写入初始文字作为占位（部分软件支持）
    // worksheet.write_string(summary_row, 0, &format!("合计人民币大写：{}", initial_chinese), &Format::new().set_font_size(11))?;

    // 数字总计公式 (SUM)
    let sum_formula = format!("=SUM({}{}:{}{})", amount_col_name, start_data_row, amount_col_name, last_data_row);
    
    let total_label_format = Format::new().set_font_size(11).set_align(FormatAlign::Right);
    worksheet.merge_range(summary_row, 4, summary_row, total_cols as u16, "", &total_label_format)?;
    // 在合并单元格的左上角写入公式
    worksheet.write_formula_with_format(
        summary_row, 
        4, 
        sum_formula.as_str(), 
        &Format::new().set_font_size(11).set_align(FormatAlign::Right).set_num_format("\"人民币小写：\"¥#,##0.00\"元\"")
    )?;

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
        "%d/%m/%Y",
        "%m/%d/%Y",
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

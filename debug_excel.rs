// 临时调试脚本 - 查看 Excel 结构
use calamine::{open_workbook_auto, Reader, Data};

fn main() {
    let path = "test-data/客户/月结客户/和信/บอะลหอป๕ตฅ2022-01-05.xls";

    let mut workbook = match open_workbook_auto(path) {
        Ok(w) => w,
        Err(e) => {
            println!("无法打开文件: {}", e);
            return;
        }
    };

    println!("工作表列表: {:?}", workbook.sheet_names());

    let sheet_name = workbook.sheet_names().first().unwrap().clone();
    let range = workbook.worksheet_range(&sheet_name).expect("无法读取工作表");

    println!("\n前 10 行数据：");
    for (idx, row) in range.rows().enumerate() {
        if idx >= 10 {
            break;
        }
        println!("行 {}: {:?}", idx, row.iter().map(|c| c.to_string()).collect::<Vec<_>>());
    }

    // 重点检查日期所在的单元格（第5行，列7/8左右）
    if let Some(row4) = range.rows().nth(4) {
        println!("\n第 5 行 (index 4) 详细内容:");
        for (col_idx, cell) in row4.iter().enumerate() {
            println!("  Col {}: {:?} (String: {})", col_idx, cell, cell.to_string());
        }
    }
}
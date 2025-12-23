// 临时调试脚本 - 查看 Excel 结构
use calamine::{open_workbook_auto, Reader, Data};

fn main() {
    let path = "/Users/bytedance/Repositories/monthlyreport/raw-data/智得兴/智得兴2023-01-04.xls";

    let mut workbook = open_workbook_auto(path).expect("无法打开文件");

    println!("工作表列表: {:?}", workbook.sheet_names());

    let sheet_name = workbook.sheet_names().first().unwrap().clone();
    let range = workbook.worksheet_range(&sheet_name).expect("无法读取工作表");

    println!("\n前 20 行数据：");
    for (idx, row) in range.rows().enumerate() {
        if idx >= 20 {
            break;
        }
        println!("行 {}: {:?}", idx, row.iter().map(|c| c.to_string()).collect::<Vec<_>>());
    }
}

use serde::{Deserialize, Serialize};

/// 送货单条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryItem {
    /// 货名
    pub product_name: String,
    /// 规格
    pub spec: String,
    /// 数量
    pub quantity: f64,
    /// 单位
    pub unit: String,
    /// 单价
    pub unit_price: f64,
    /// 金额
    pub amount: f64,
    /// 客户
    pub customer: String,
    /// 日期
    pub date: String,
    /// 送货单号
    pub delivery_order_no: String,
    /// 订单号 (PO No)
    pub order_no: String,
    /// 源文件
    pub source_file: String,
    /// 客户类型 (monthly: 月结, cash: 现金)
    #[serde(default = "default_customer_type")]
    pub customer_type: String,
}

fn default_customer_type() -> String {
    "monthly".to_string()
}

/// 汇总数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryItem {
    pub product_name: String,
    pub spec: String,
    pub unit: String,
    pub quantity: f64,
    pub average_price: f64,
    pub amount: f64,
    pub customers: String,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 公司名称
    pub company_name: String,
    /// 地址
    pub address: String,
    /// 电话
    pub phone: String,
    /// 传真
    pub fax: String,
    /// 原始数据路径
    pub raw_data_path: String,
    /// 输出路径
    pub output_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            company_name: "百惠行对账单".to_string(),
            address: "东莞市黄江镇华南塑胶城区132号".to_string(),
            phone: "(0769) 83631717".to_string(),
            fax: "83637787".to_string(),
            raw_data_path: "raw-data".to_string(),
            output_path: "output".to_string(),
        }
    }
}

/// 进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub step: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

/// 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub success: bool,
    pub message: String,
    pub generated_count: usize,
    pub skipped_count: usize,
    pub output_path: String,
}

/// 文件验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileValidationError {
    pub file: String,
    pub error: String,
}

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub success: bool,
    pub message: String,
    pub total_files: usize,
    pub valid_files: usize,
    pub errors: Vec<FileValidationError>,
    pub warnings: Vec<FileValidationError>,
    pub items: Vec<DeliveryItem>,
}

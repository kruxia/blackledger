use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    50
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.page_size.min(1000) as i64
    }

    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) * self.page_size) as i64
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub page_size: u32,
    pub total: Option<i64>,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, params: &PaginationParams, total: Option<i64>) -> Self {
        let has_more = data.len() as u32 >= params.page_size;

        Self {
            data,
            pagination: PaginationMeta {
                page: params.page,
                page_size: params.page_size,
                total,
                has_more,
            },
        }
    }
}

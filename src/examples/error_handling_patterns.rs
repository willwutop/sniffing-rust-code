use std::error::Error;
use std::io;

// 实现自定义错误类型
#[derive(Debug)]
struct MyError(String, Box<dyn Error>);

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyError occurred: {}", &self.0)
    }
}

impl Error for MyError {
    // 💡 实现错误链
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.1)
    }
}

// 通过enum实现一个场景中完整的错误类型
#[derive(Debug)]
pub enum QueryError {
    NotFound,
    ParamError(String),
    IOError(io::Error),
    // ...
    OtherError(String), // 处理未预见的其他错误
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::NotFound => write!(f, "未找到数据"),
            QueryError::ParamError(msg) => write!(f, "参数错误: {}", msg),
            QueryError::IOError(e) => write!(f, "文件错误: {}", e),
            QueryError::OtherError(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl Error for QueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            QueryError::IOError(e) => Some(e),
            _ => None,
        }
    }
}

// Error使用From trait进行类型转换
#[derive(Debug)]
enum FileError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
    Validation(String),
}

// 实现From trait后，?操作符会自动转换
impl From<std::io::Error> for FileError {
    fn from(error: std::io::Error) -> Self {
        FileError::Io(error)
    }
}

impl From<std::num::ParseIntError> for FileError {
    fn from(error: std::num::ParseIntError) -> Self {
        FileError::Parse(error)
    }
}

fn read_and_parse_number(path: &str) -> Result<i32, FileError> {
    let content = std::fs::read_to_string(path)?; // 👈 io::Error -> FileError
    let number = content.trim().parse()?; // 👈 ParseIntError -> FileError

    if number < 0 {
        return Err(FileError::Validation("数字不能为负".to_string()));
    }

    Ok(number)
}

// 永远不要使用字符串作为错误类型
// ❌ 字符串错误的问题
fn parse_number_bad(s: &str) -> Result<i32, &'static str> {
    if s.is_empty() {
        return Err("输入为空");
    }
    s.parse().map_err(|e| "解析失败：{e:?}") // 💥 没有具体的错误类型，通过字符串判断效率低下
}

// ✅ 使用具体错误类型的优势
#[derive(Debug)]
enum ParseError {
    Empty,
    InvalidFormat(std::num::ParseIntError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "输入不能为空"),
            ParseError::InvalidFormat(e) => write!(f, "数字格式错误: {}", e),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Empty => None,
            ParseError::InvalidFormat(e) => Some(e), // 👈 保持错误链
        }
    }
}

fn parse_number_good(s: &str) -> Result<i32, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }
    s.parse().map_err(ParseError::InvalidFormat)
}

mod use_thiserror {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum DatabaseError {
        #[error("连接错误: {message}")] //  👈 定义错误格式化
        Connection { message: String },

        #[error("查询错误: {query}")]
        Query { query: String },

        #[error("IO错误")]
        Io(#[from] std::io::Error), // 👈 #[from] 自动实现From trait

        #[error("序列化错误")]
        Serialization(#[from] serde_json::Error),
    }

    // 使用示例
    fn execute_query(query: &str) -> Result<String, DatabaseError> {
        if query.trim().is_empty() {
            return Err(DatabaseError::Query {
                query: query.to_string(),
            });
        }

        // 模拟查询执行
        let result = std::fs::read_to_string("result.json")?; // 自动转换IO错误
        let parsed: serde_json::Value = serde_json::from_str(&result)?; // 自动转换JSON错误

        Ok(parsed.to_string())
    }
}

mod use_anyhow {
    use anyhow::{bail, Context, Result};

    fn process_user_data(user_id: u32) -> Result<UserProfile> {
        if user_id == 0 {
            bail!("用户ID不能为0"); // 👈 直接生成错误
        }

        let user = fetch_user(user_id).with_context(|| format!("获取用户{}失败", user_id))?; // 👈 添加错误上下文信息

        let profile = build_profile(&user).context("构建用户档案失败")?; // 👈 添加错误上下文信息

        Ok(profile)
    }

    fn fetch_user(user_id: u32) -> Result<User> {
        // 模拟数据库查询
        if user_id == 999 {
            bail!("用户{}不存在", user_id);
        }

        Ok(User {
            id: user_id,
            name: format!("用户{}", user_id),
            email: format!("user{}@example.com", user_id),
        })
    }

    fn build_profile(user: &User) -> Result<UserProfile> {
        if user.email.is_empty() {
            bail!("用户邮箱为空");
        }

        Ok(UserProfile {
            user_id: user.id,
            display_name: user.name.clone(),
            avatar_url: format!("https://avatar.example.com/{}", user.id),
        })
    }

    #[derive(Debug)]
    struct User {
        id: u32,
        name: String,
        email: String,
    }

    #[derive(Debug)]
    struct UserProfile {
        user_id: u32,
        display_name: String,
        avatar_url: String,
    }
}

mod error_layout {
    use thiserror::Error;

    // 1. 业务层错误
    #[derive(Error, Debug)]
    pub enum UserDomainError {
        #[error("用户不存在: {user_id}")]
        NotFound { user_id: u32 },

        #[error("用户已存在: {email}")]
        AlreadyExists { email: String },

        #[error("验证失败: {field} - {reason}")]
        ValidationFailed { field: String, reason: String },
    }

    // 2. 基础设施层错误
    #[derive(Error, Debug)]
    pub enum InfrastructureError {
        #[error("数据库连接失败")]
        Database(#[from] sqlx::Error),

        #[error("Redis连接失败")]
        Redis(#[from] redis::RedisError),

        #[error("HTTP请求失败")]
        Http(#[from] reqwest::Error),
    }

    // 3. 应用层统一错误
    #[derive(Error, Debug)]
    pub enum AppError {
        #[error("业务逻辑错误")]
        Domain(#[from] UserDomainError),

        #[error("基础设施错误")]
        Infrastructure(#[from] InfrastructureError),

        #[error("内部服务器错误")]
        Internal(#[from] anyhow::Error),
    }

    // 4. HTTP响应转换
    use axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
        Json,
    };
    use serde_json::json;

    impl IntoResponse for AppError {
        fn into_response(self) -> Response {
            let (status, message, code) = match &self {
                AppError::Domain(UserDomainError::NotFound { .. }) => {
                    (StatusCode::NOT_FOUND, "用户不存在", "USER_NOT_FOUND")
                }
                AppError::Domain(UserDomainError::AlreadyExists { .. }) => {
                    (StatusCode::CONFLICT, "用户已存在", "USER_ALREADY_EXISTS")
                }
                AppError::Domain(UserDomainError::ValidationFailed { .. }) => {
                    (StatusCode::BAD_REQUEST, "输入验证失败", "VALIDATION_FAILED")
                }
                AppError::Infrastructure(_) => {
                    // 🚨 不向用户暴露基础设施错误详情
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "服务暂时不可用",
                        "SERVICE_UNAVAILABLE",
                    )
                }
                AppError::Internal(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "内部服务器错误",
                    "INTERNAL_ERROR",
                ),
            };

            let body = Json(json!({
                "error": {
                    "code": code,
                    "message": message
                }
            }));

            (status, body).into_response()
        }
    }
}

mod error_bad_practice {
    use anyhow::Context;
    use reqwest::Url;
    use std::fs;

    fn bad_unwrap() {
        // ❌ 早期的我经常这样写
        let config = fs::read_to_string("config.json").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&config).expect("解析失败");

        // ✅ 确保Result为ok，否则应该panic
        let default_addr = "[::]:8080"; // 🚨 从配置文件中读取的地址
        let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| default_addr.to_string());
        let addr = Url::parse(&addr).expect("地址解析失败"); // 解析失败程序无法运行，应该panic
    }

    // ❌ 无法选择正确的错误
    // fn mixed_errors() -> Result<String, ???> {  // 🚨 不知道返回什么错误类型
    //     let file_content = std::fs::read_to_string("data.json")?;  // io::Error
    //     let value: serde_json::Value = serde_json::from_str(&file_content)?;  // serde_json::Error
    //     Ok(value.to_string())
    // }

    // ✅ 使用anyhow后变得简单
    fn mixed_errors_fixed() -> anyhow::Result<String> {
        let file_content = std::fs::read_to_string("data.json")?;
        let value: serde_json::Value = serde_json::from_str(&file_content)?;
        Ok(value.to_string())
    }

    // ❌ 错误信息不够详细，调试时很痛苦
    fn process_files(paths: &[&str]) -> anyhow::Result<()> {
        for path in paths {
            let content = std::fs::read_to_string(path)?; // 💥 哪个文件出错了？
            process_content(&content)?; // 💥 处理哪一步失败了？
        }
        Ok(())
    }

    // ✅ 添加上下文信息，调试变得轻松
    fn process_files_with_context(paths: &[&str]) -> anyhow::Result<()> {
        for path in paths {
            let content =
                std::fs::read_to_string(path).with_context(|| format!("读取文件失败: {}", path))?;
            process_content(&content).with_context(|| format!("处理文件内容失败: {}", path))?;
        }
        Ok(())
    }

    fn process_content(_content: &str) -> anyhow::Result<()> {
        // 模拟处理逻辑
        Ok(())
    }
}

// 库开发：使用thiserror
pub mod my_library {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum LibraryError {
        #[error("配置错误: {0}")]
        Config(String),

        #[error("IO错误")]
        Io(#[from] std::io::Error),
    }

    pub fn library_function() -> Result<(), LibraryError> {
        Err(LibraryError::Config("缺少必要配置".to_string()))
    }
}

// 应用开发：使用anyhow
mod my_application {
    use super::my_library;
    use anyhow::{Context, Result};

    pub fn application_function() -> Result<()> {
        my_library::library_function().context("调用库函数失败")?;
        Ok(())
    }
}

mod error_retry {
    use anyhow::Result;
    use std::time::Duration;

    // 简单的重试机制
    async fn with_retry<F, T>(mut operation: F, max_retries: usize) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    // 使用示例
    async fn fetch_data_with_retry() -> Result<String> {
        with_retry(
            || {
                // 模拟可能失败的操作
                if rand::random::<bool>() {
                    Ok("数据获取成功".to_string())
                } else {
                    anyhow::bail!("网络错误")
                }
            },
            3,
        )
        .await
    }
}

// 开启backtrace获取详细错误信息
// 自定义错误报告格式
// RUST_BACKTRACE=1
fn report_error(e: &anyhow::Error) {
    eprintln!("❌ 错误: {}", e);
    for (i, cause) in e.chain().skip(1).enumerate() {
        eprintln!("   原因 {}: {}", i + 1, cause);
    }
}

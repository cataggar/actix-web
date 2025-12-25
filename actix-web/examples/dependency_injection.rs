//! Example demonstrating dependency injection with fundle, error handling with ohno,
//! and database access with sea-orm.
//!
//! This example shows:
//! - Using fundle for compile-time safe dependency injection
//! - Using ohno for error handling with automatic backtrace capture
//! - Using sea-orm with SQLite (for development/testing) and PostgreSQL (for production)
//! - Integration with actix-web for building REST APIs
//!
//! ## Database Configuration
//!
//! By default, this example uses SQLite for easy local development:
//! - Database file: `./example.db`
//!
//! To use PostgreSQL instead, set the DATABASE_URL environment variable:
//! ```sh
//! DATABASE_URL=postgres://user:password@localhost/dbname cargo run --example dependency_injection
//! ```
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example dependency_injection
//! ```
//!
//! Then test with curl:
//! ```sh
//! # Create a user
//! curl -X POST http://localhost:8080/users -H "Content-Type: application/json" -d '{"name":"Alice","email":"alice@example.com"}'
//!
//! # List all users
//! curl http://localhost:8080/users
//!
//! # Get a specific user
//! curl http://localhost:8080/users/1
//!
//! # Update a user
//! curl -X PUT http://localhost:8080/users/1 -H "Content-Type: application/json" -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
//!
//! # Delete a user
//! curl -X DELETE http://localhost:8080/users/1
//! ```

use actix_web::{delete, get, post, put, web, App, HttpResponse, HttpServer, Responder};
use fundle::bundle;
use sea_orm::{ActiveModelTrait, ActiveValue, Database, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

// ============================================================================
// Error Handling
// ============================================================================

/// Application error type with custom error handling
/// 
/// This example demonstrates error handling patterns. While we include ohno
/// in dependencies to show it can be used, we use a simpler approach here
/// for clarity in the example.
#[derive(Debug)]
pub enum AppError {
    Database { message: String },
    NotFound { id: i32 },
    InvalidInput { message: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database { message } => write!(f, "Database error: {}", message),
            AppError::NotFound { id } => write!(f, "User not found with id: {}", id),
            AppError::InvalidInput { message } => write!(f, "Invalid input: {}", message),
        }
    }
}

impl std::error::Error for AppError {}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound { .. } => {
                HttpResponse::NotFound().json(serde_json::json!({"error": self.to_string()}))
            }
            AppError::InvalidInput { .. } => {
                HttpResponse::BadRequest().json(serde_json::json!({"error": self.to_string()}))
            }
            AppError::Database { .. } => HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": self.to_string()})),
        }
    }
}

// ============================================================================
// Database Entity Models with sea-orm
// ============================================================================

pub mod entity {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub email: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// DTO Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserDto {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<entity::Model> for UserResponse {
    fn from(model: entity::Model) -> Self {
        UserResponse {
            id: model.id,
            name: model.name,
            email: model.email,
        }
    }
}

// ============================================================================
// Database Service Layer
// ============================================================================

/// Trait defining user service operations
/// 
/// This trait allows for dependency injection of the service itself,
/// making the code more testable and following SOLID principles.
pub trait UserServiceTrait: Send + Sync {
    fn create_user(
        &self,
        dto: CreateUserDto,
    ) -> impl std::future::Future<Output = Result<entity::Model, AppError>> + Send;

    fn get_all_users(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<entity::Model>, AppError>> + Send;

    fn get_user_by_id(
        &self,
        id: i32,
    ) -> impl std::future::Future<Output = Result<entity::Model, AppError>> + Send;

    fn update_user(
        &self,
        id: i32,
        dto: UpdateUserDto,
    ) -> impl std::future::Future<Output = Result<entity::Model, AppError>> + Send;

    fn delete_user(
        &self,
        id: i32,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send;
}

/// Implementation of UserService with injected database connection
/// 
/// Note: DatabaseConnection is a handle to a connection pool, not a single connection.
/// Cloning it is cheap and creates another reference to the same underlying pool,
/// not a new pool. This is safe for concurrent use across multiple services/threads.
#[derive(Clone)]
pub struct UserService {
    db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl UserServiceTrait for UserService {
    async fn create_user(&self, dto: CreateUserDto) -> Result<entity::Model, AppError> {
        log::info!("Creating user: {}", dto.name);
        
        let user = entity::ActiveModel {
            name: ActiveValue::Set(dto.name),
            email: ActiveValue::Set(dto.email),
            ..Default::default()
        };

        user.insert(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    async fn get_all_users(&self) -> Result<Vec<entity::Model>, AppError> {
        log::info!("Getting all users");
        
        entity::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    async fn get_user_by_id(&self, id: i32) -> Result<entity::Model, AppError> {
        log::info!("Getting user by id: {}", id);
        
        entity::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })?
            .ok_or_else(|| AppError::NotFound { id })
    }

    async fn update_user(&self, id: i32, dto: UpdateUserDto) -> Result<entity::Model, AppError> {
        log::info!("Updating user: {}", id);
        
        let user = self.get_user_by_id(id).await?;

        let mut user: entity::ActiveModel = user.into();
        user.name = ActiveValue::Set(dto.name);
        user.email = ActiveValue::Set(dto.email);

        user.update(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    async fn delete_user(&self, id: i32) -> Result<(), AppError> {
        log::info!("Deleting user: {}", id);
        
        let user = self.get_user_by_id(id).await?;
        let user: entity::ActiveModel = user.into();

        user.delete(&self.db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })?;

        Ok(())
    }
}

// ============================================================================
// Application State with fundle for Dependency Injection
// ============================================================================

/// Application state bundle managed by fundle
/// 
/// This demonstrates dependency injection where the service itself is injected,
/// not just individual dependencies. The UserService already has the database
/// connection injected into it, showcasing a layered DI approach.
#[bundle]
#[derive(Clone)]
pub struct AppState {
    /// User service with injected database connection
    pub user_service: UserService,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

#[post("/users")]
async fn create_user(
    state: web::Data<AppState>,
    dto: web::Json<CreateUserDto>,
) -> Result<impl Responder, AppError> {
    let user = state.user_service.create_user(dto.into_inner()).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[get("/users")]
async fn list_users(state: web::Data<AppState>) -> Result<impl Responder, AppError> {
    let users = state.user_service.get_all_users().await?;
    let response: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(web::Json(response))
}

#[get("/users/{id}")]
async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let user = state.user_service.get_user_by_id(id).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[put("/users/{id}")]
async fn update_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
    dto: web::Json<UpdateUserDto>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let user = state.user_service.update_user(id, dto.into_inner()).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[delete("/users/{id}")]
async fn delete_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    state.user_service.delete_user(id).await?;
    Ok(HttpResponse::NoContent())
}

// ============================================================================
// Database Initialization
// ============================================================================

async fn setup_database(db: &DatabaseConnection) -> Result<(), AppError> {
    use sea_orm::{ConnectionTrait, DbBackend};

    let backend = db.get_database_backend();
    
    // For SQLite, we need to create the table directly
    let create_table_sql = match backend {
        DbBackend::Sqlite => {
            r#"CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                name TEXT NOT NULL,
                email TEXT NOT NULL
            )"#
        }
        DbBackend::Postgres => {
            r#"CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY NOT NULL,
                name VARCHAR NOT NULL,
                email VARCHAR NOT NULL
            )"#
        }
        _ => {
            return Err(AppError::Database {
                message: "Unsupported database backend".to_string(),
            });
        }
    };

    db.execute_unprepared(create_table_sql)
        .await
        .map_err(|e| AppError::Database {
            message: format!("Failed to create table: {}", e),
        })?;

    Ok(())
}

// ============================================================================
// Main Application Entry Point
// ============================================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Determine database URL from environment or use SQLite as default
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let url = "sqlite://./example.db?mode=rwc";
        log::info!("Using SQLite database: {}", url);
        url.to_string()
    });

    if database_url.starts_with("postgres") {
        log::info!("Using PostgreSQL database");
    }

    // Connect to database
    // DatabaseConnection is a handle to a connection pool (managed by SQLx under the hood).
    // The default pool configuration is used here, but you can customize it with ConnectOptions
    // to set max_connections, min_connections, connect_timeout, etc.
    let db = Database::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Setup database schema
    setup_database(&db)
        .await
        .expect("Failed to setup database");

    // Create user service with injected database connection
    // The DatabaseConnection can be cloned cheaply - all clones share the same connection pool
    let user_service = UserService::new(db);

    // Build application state using fundle
    // This demonstrates dependency injection where the service itself is injected.
    // The closure `|_| user_service.clone()` is the fundle pattern for setting a field
    // that doesn't depend on other fields in the builder.
    // Cloning user_service (which contains a DatabaseConnection) is safe and efficient.
    let state = AppState::builder()
        .user_service(|_| user_service.clone())
        .build();

    log::info!("Starting HTTP server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(create_user)
            .service(list_users)
            .service(get_user)
            .service(update_user)
            .service(delete_user)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

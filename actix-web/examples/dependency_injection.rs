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
// Application State with fundle for Dependency Injection
// ============================================================================

/// Application state bundle managed by fundle
#[bundle]
#[derive(Clone)]
pub struct AppState {
    /// Database connection managed by sea-orm
    pub db: DatabaseConnection,
}

// ============================================================================
// Database Service Layer
// ============================================================================

pub struct UserService;

impl UserService {
    pub async fn create_user(
        db: &DatabaseConnection,
        dto: CreateUserDto,
    ) -> Result<entity::Model, AppError> {
        log::info!("Creating user: {}", dto.name);
        
        let user = entity::ActiveModel {
            name: ActiveValue::Set(dto.name),
            email: ActiveValue::Set(dto.email),
            ..Default::default()
        };

        user.insert(db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    pub async fn get_all_users(
        db: &DatabaseConnection,
    ) -> Result<Vec<entity::Model>, AppError> {
        log::info!("Getting all users");
        
        entity::Entity::find()
            .all(db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    pub async fn get_user_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<entity::Model, AppError> {
        log::info!("Getting user by id: {}", id);
        
        entity::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })?
            .ok_or_else(|| AppError::NotFound { id })
    }

    pub async fn update_user(
        db: &DatabaseConnection,
        id: i32,
        dto: UpdateUserDto,
    ) -> Result<entity::Model, AppError> {
        log::info!("Updating user: {}", id);
        
        let user = Self::get_user_by_id(db, id).await?;

        let mut user: entity::ActiveModel = user.into();
        user.name = ActiveValue::Set(dto.name);
        user.email = ActiveValue::Set(dto.email);

        user.update(db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })
    }

    pub async fn delete_user(db: &DatabaseConnection, id: i32) -> Result<(), AppError> {
        log::info!("Deleting user: {}", id);
        
        let user = Self::get_user_by_id(db, id).await?;
        let user: entity::ActiveModel = user.into();

        user.delete(db)
            .await
            .map_err(|e| AppError::Database {
                message: e.to_string(),
            })?;

        Ok(())
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

#[post("/users")]
async fn create_user(
    state: web::Data<AppState>,
    dto: web::Json<CreateUserDto>,
) -> Result<impl Responder, AppError> {
    let user = UserService::create_user(&state.db, dto.into_inner()).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[get("/users")]
async fn list_users(state: web::Data<AppState>) -> Result<impl Responder, AppError> {
    let users = UserService::get_all_users(&state.db).await?;
    let response: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(web::Json(response))
}

#[get("/users/{id}")]
async fn get_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let user = UserService::get_user_by_id(&state.db, id).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[put("/users/{id}")]
async fn update_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
    dto: web::Json<UpdateUserDto>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    let user = UserService::update_user(&state.db, id, dto.into_inner()).await?;
    Ok(web::Json(UserResponse::from(user)))
}

#[delete("/users/{id}")]
async fn delete_user(
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> Result<impl Responder, AppError> {
    let id = path.into_inner();
    UserService::delete_user(&state.db, id).await?;
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
    let db = Database::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Setup database schema
    setup_database(&db)
        .await
        .expect("Failed to setup database");

    // Build application state using fundle
    // The closure `|_| db.clone()` is the fundle pattern for setting a field
    // that doesn't depend on other fields in the builder. The underscore
    // indicates we're not using the builder reference.
    let state = AppState::builder().db(|_| db.clone()).build();

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

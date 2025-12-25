# Dependency Injection Example with fundle, ohno, and sea-orm

This example demonstrates a complete REST API built with actix-web that showcases:

- **Dependency Injection**: Using the `fundle` crate for compile-time safe dependency injection with trait-based service abstraction
- **Error Handling**: Custom error handling patterns inspired by the `ohno` crate
- **Database Access**: Using `sea-orm` ORM for async database operations
- **Multi-Database Support**: SQLite for development/testing and PostgreSQL for production

## Features

- Full CRUD operations (Create, Read, Update, Delete) for a User entity
- Trait-based service layer with injected dependencies
- Type-safe dependency injection with `fundle`
- Clean error handling with custom error types
- Async database operations with `sea-orm`
- Automatic database schema creation
- Environment-based database configuration

## Dependencies

The example uses these key crates:

- **`fundle`** (v0.3): Compile-time safe dependency injection framework
- **`ohno`** (v0.2): Error handling library (included in dependencies to demonstrate integration)
- **`sea-orm`** (v2.0.0-rc.22): Async ORM with SQLite and PostgreSQL support
- **`actix-web`** (v4): The web framework

## Database Configuration

### SQLite (Default for Development)

By default, the example uses SQLite with a local file database:

```bash
cargo run --example dependency_injection
```

This creates a `example.db` file in the current directory.

### PostgreSQL (Production)

To use PostgreSQL, set the `DATABASE_URL` environment variable:

```bash
# Make sure PostgreSQL is running and you have created a database
DATABASE_URL=postgres://username:password@localhost/dbname cargo run --example dependency_injection
```

## API Endpoints

### Create User
```bash
curl -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

Response:
```json
{"id":1,"name":"Alice","email":"alice@example.com"}
```

### List All Users
```bash
curl http://localhost:8080/users
```

Response:
```json
[
  {"id":1,"name":"Alice","email":"alice@example.com"},
  {"id":2,"name":"Bob","email":"bob@example.com"}
]
```

### Get User by ID
```bash
curl http://localhost:8080/users/1
```

Response:
```json
{"id":1,"name":"Alice","email":"alice@example.com"}
```

### Update User
```bash
curl -X PUT http://localhost:8080/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
```

Response:
```json
{"id":1,"name":"Alice Smith","email":"alice.smith@example.com"}
```

### Delete User
```bash
curl -X DELETE http://localhost:8080/users/1
```

Response: 204 No Content

### Error Handling Example
```bash
curl http://localhost:8080/users/999
```

Response:
```json
{"error":"User not found with id: 999"}
```

## Code Structure

The example is organized into several key sections:

1. **Error Handling**: Custom `AppError` type that implements `actix_web::ResponseError`
2. **Database Entities**: Sea-ORM entity definitions for the User model
3. **DTOs**: Data Transfer Objects for API requests/responses
4. **Service Layer**: `UserServiceTrait` and `UserService` with injected dependencies
5. **Application State**: `AppState` bundle managed by fundle for dependency injection
6. **HTTP Handlers**: Actix-web route handlers that use the injected service
7. **Database Setup**: Automatic schema creation supporting both SQLite and PostgreSQL

## Key Concepts Demonstrated

### Dependency Injection with fundle

The example demonstrates a layered dependency injection approach:

1. **Service with Injected Dependencies**: The `UserService` has the database connection injected:

```rust
pub struct UserService {
    db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
```

2. **Trait-Based Abstraction**: Services implement traits for better testability:

```rust
pub trait UserServiceTrait: Send + Sync {
    fn create_user(&self, dto: CreateUserDto) 
        -> impl Future<Output = Result<entity::Model, AppError>> + Send;
    // ... other methods
}

impl UserServiceTrait for UserService {
    // Implementation with access to self.db
}
```

3. **Application State Bundle**: The service itself is injected into the application state:

```rust
#[bundle]
#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
}

// Usage in main:
let user_service = UserService::new(db);
let state = AppState::builder()
    .user_service(|_| user_service.clone())
    .build();
```

This approach provides:
- Compile-time verification of all dependencies
- Easy testing through trait mocking
- Clean separation of concerns
- No runtime dependency injection overhead

### Error Handling

Custom error types provide clear, structured error responses:

```rust
#[derive(Debug)]
pub enum AppError {
    Database { message: String },
    NotFound { id: i32 },
    InvalidInput { message: String },
}
```

These errors are automatically converted to appropriate HTTP responses through the `ResponseError` trait.

### Async Database Operations

All database operations are fully async, leveraging sea-orm's async capabilities. The service methods have access to the injected database connection through `self.db`:

```rust
async fn create_user(&self, dto: CreateUserDto) -> Result<entity::Model, AppError> {
    let user = entity::ActiveModel { /* ... */ };
    user.insert(&self.db).await?
}
```

No blocking operations or thread pools needed - everything works seamlessly with actix-web's async runtime.

### Connection Pooling

**Important**: `DatabaseConnection` is a handle to a connection pool, not a single database connection.

- **Cloning is cheap**: When you clone a `DatabaseConnection`, you're creating another reference to the *same* underlying pool, not creating a new pool.
- **Shared pool**: All clones share the same connection pool configuration (max connections, timeouts, etc.).
- **Thread-safe**: Safe to clone and share across services, workers, and async tasks.
- **Automatic management**: Connections are automatically acquired from the pool when needed and returned after use.

Example with custom pool configuration:
```rust
use sea_orm::ConnectOptions;
use std::time::Duration;

let mut opt = ConnectOptions::new("******localhost/mydb");
opt.max_connections(100)
   .min_connections(5)
   .connect_timeout(Duration::from_secs(8))
   .idle_timeout(Duration::from_secs(300));

let db = Database::connect(opt).await?;
```

In this example, we use the default pool settings, which are suitable for most use cases.

## Testing

To test the example:

1. Start the server:
   ```bash
   cargo run --example dependency_injection
   ```

2. In another terminal, run the API tests:
   ```bash
   # Create users
   curl -X POST http://localhost:8080/users -H "Content-Type: application/json" \
     -d '{"name":"Alice","email":"alice@example.com"}'
   curl -X POST http://localhost:8080/users -H "Content-Type: application/json" \
     -d '{"name":"Bob","email":"bob@example.com"}'
   
   # List users
   curl http://localhost:8080/users
   
   # Get specific user
   curl http://localhost:8080/users/1
   
   # Update user
   curl -X PUT http://localhost:8080/users/1 -H "Content-Type: application/json" \
     -d '{"name":"Alice Smith","email":"alice.smith@example.com"}'
   
   # Delete user
   curl -X DELETE http://localhost:8080/users/2
   
   # Test error handling
   curl http://localhost:8080/users/999
   ```

## Production Deployment

For production deployment with PostgreSQL:

1. Set up a PostgreSQL database
2. Configure the DATABASE_URL environment variable
3. The application will automatically create the required tables on startup
4. Consider using connection pooling settings appropriate for your load

Example production configuration:
```bash
export DATABASE_URL="postgres://user:password@db.example.com:5432/production_db"
cargo run --release --example dependency_injection
```

## Further Reading

- [fundle documentation](https://docs.rs/fundle/)
- [ohno documentation](https://docs.rs/ohno/)
- [sea-orm documentation](https://www.sea-ql.org/SeaORM/)
- [actix-web documentation](https://actix.rs)

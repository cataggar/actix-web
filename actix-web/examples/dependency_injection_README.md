# Dependency Injection Example with fundle, ohno, and sea-orm

This example demonstrates a complete REST API built with actix-web that showcases:

- **Dependency Injection**: Using the `fundle` crate for compile-time safe dependency injection
- **Error Handling**: Custom error handling patterns inspired by the `ohno` crate
- **Database Access**: Using `sea-orm` ORM for async database operations
- **Multi-Database Support**: SQLite for development/testing and PostgreSQL for production

## Features

- Full CRUD operations (Create, Read, Update, Delete) for a User entity
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
4. **Application State**: `AppState` bundle managed by fundle for dependency injection
5. **Service Layer**: `UserService` with business logic and database operations
6. **HTTP Handlers**: Actix-web route handlers that use the service layer
7. **Database Setup**: Automatic schema creation supporting both SQLite and PostgreSQL

## Key Concepts Demonstrated

### Dependency Injection with fundle

The `AppState` struct is annotated with `#[bundle]` to enable fundle's dependency injection:

```rust
#[bundle]
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
```

This generates a type-safe builder that ensures all dependencies are properly initialized at compile-time.

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

All database operations are fully async, leveraging sea-orm's async capabilities:

```rust
pub async fn create_user(
    db: &DatabaseConnection,
    dto: CreateUserDto,
) -> Result<entity::Model, AppError>
```

No blocking operations or thread pools needed - everything works seamlessly with actix-web's async runtime.

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

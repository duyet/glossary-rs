# CLAUDE.md - Development Guide & Architecture

This document captures the architecture, design principles, and development guidelines for the Glossary service. It serves as a reference for maintaining code quality and consistency.

## 🎯 Project Vision

**Glossary** is a production-ready knowledge management system that demonstrates:
- **Correctness first** - Strong types prevent entire classes of bugs
- **Explicit over implicit** - Every data transformation is visible
- **Database integrity** - PostgreSQL enforces referential integrity
- **Semantic HTTP** - Status codes tell the truth
- **Beautiful UX** - Modern interface with dark mode

## 🏗️ Architecture

### Layered Design

```
┌─────────────────────────────────────┐
│  Frontend (static/)                 │
│  - Vanilla JS (zero dependencies)  │
│  - CSS custom properties (theming) │
│  - Progressive enhancement         │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  HTTP Layer (main.rs)               │
│  - Actix-web handlers              │
│  - Security headers middleware     │
│  - CORS configuration              │
│  - Static file serving             │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  API Layer (v1/)                    │
│  - Endpoint handlers               │
│  - Request/Response DTOs           │
│  - Semantic error conversion       │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  Business Logic                     │
│  - CRUD operations                 │
│  - Search functionality            │
│  - Like system                     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  Database Layer (Diesel ORM)        │
│  - Type-safe queries               │
│  - Migrations                      │
│  - Schema definitions              │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  PostgreSQL Database                │
│  - CASCADE constraints             │
│  - Performance indexes             │
│  - Audit history                   │
└─────────────────────────────────────┘
```

### Data Flow

**Request Path:**
```
HTTP Request → Actix Handler → web::block() → Diesel Query → PostgreSQL
```

**Response Path:**
```
PostgreSQL → Diesel Result → ApiError/Success → JSON Response
```

### Type System

We maintain three distinct types for glossary data:

1. **`GlossaryRequest`** - API input (validated, sanitized)
2. **`GlossaryDB`** - Database model (Diesel schema)
3. **`Glossary`** - API output (enriched with metadata)

**Never mix these types.** Each serves a specific purpose:
- Input types have HTML sanitization (Ammonia)
- DB types have exact PostgreSQL field mappings
- Output types include likes, history, and computed fields

## ⚡ Error Handling Philosophy

### Semantic HTTP Status Codes

Every error must return the **semantically correct** status code:

```rust
// ✅ CORRECT
diesel::delete(glossary.find(id))
    .execute(conn)?  // → Automatic conversion
// NotFound → 404
// UniqueViolation → 409
// ForeignKeyViolation → 409
// Other errors → 500

// ❌ WRONG - Don't do this
Err(ErrorResp::new(&e.to_string()))  // Always 400 BAD_REQUEST
```

### Error Type Hierarchy

```rust
pub enum ApiError {
    NotFound(String),           // 404
    InvalidInput(String),       // 400
    Conflict(String),           // 409
    UnprocessableEntity(String),// 422
    InternalError(String),      // 500
    DatabaseError(String),      // 500
}
```

**Automatic Diesel conversion:**
```rust
impl From<DieselError> for ApiError {
    fn from(error: DieselError) -> Self {
        match error {
            DieselError::NotFound => ApiError::NotFound(...),
            DieselError::DatabaseError(UniqueViolation, _) => ApiError::Conflict(...),
            DieselError::DatabaseError(ForeignKeyViolation, _) => ApiError::Conflict(...),
            _ => ApiError::InternalError(...)
        }
    }
}
```

**Usage in handlers:**
```rust
// The `??` operator automatically converts errors
let glossary = web::block(move || get_glossary(&mut conn, id)).await??;
```

## 🗄️ Database Design Principles

### 1. CASCADE Constraints for Referential Integrity

**Always use ON DELETE CASCADE** for child tables:

```sql
-- ✅ CORRECT
ALTER TABLE likes
ADD CONSTRAINT likes_glossary_id_fkey
    FOREIGN KEY (glossary_id)
    REFERENCES glossary(id)
    ON DELETE CASCADE;
```

**Why:** PostgreSQL guarantees atomic deletion. No orphaned records. No manual cascade logic.

### 2. Performance Indexes

Add indexes on:
- Foreign keys (e.g., `glossary_id`)
- Frequently queried columns (e.g., `term`, `created_at`)
- Sort columns (e.g., `created_at DESC`)
- Composite indexes for common queries

```sql
CREATE INDEX idx_glossary_term ON glossary(term);
CREATE INDEX idx_likes_glossary_id_created_at ON likes(glossary_id, created_at DESC);
```

### 3. Immutable Audit History

The `glossary_history` table is **append-only**:
- Never UPDATE or DELETE history records
- Each edit creates a new history entry
- `revision` number tracks changes
- `who` field captures the authenticated user

### 4. Unique Constraints

Terms are unique (case-sensitive):
```sql
ALTER TABLE glossary ADD CONSTRAINT glossary_term_unique UNIQUE (term);
```

Returns `409 CONFLICT` when violated (via ApiError::Conflict).

## 🔒 Security Principles

### 1. Security Headers

All responses include:
```
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'; ...
Referrer-Policy: strict-origin-when-cross-origin
```

### 2. HTML Sanitization

Use Ammonia for all user-provided HTML/text:

```rust
fn sanitize_html<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(clean(&s))
}

#[derive(Deserialize)]
pub struct GlossaryRequest {
    #[serde(deserialize_with = "sanitize_html")]
    pub term: String,

    #[serde(deserialize_with = "sanitize_html")]
    pub definition: String,
}
```

### 3. Authentication Header

The service expects authentication to be handled by a reverse proxy:
```
x-authenticated-user-email: user@example.com
```

This header is captured for audit history (`who` field).

## 🎨 Frontend Principles

### 1. Zero Dependencies

The frontend uses **vanilla JavaScript** with:
- No build tools required
- Modern ES6+ features
- Progressive enhancement
- Embedded directly in binary via `include_str!()`

### 2. CSS Custom Properties for Theming

```css
:root {
    --color-bg: #ffffff;
    --color-primary: #0d6efd;
}

[data-theme="dark"] {
    --color-bg: #1a1a1a;
    --color-primary: #4dabf7;
}
```

Toggle theme with `localStorage` persistence.

### 3. API Error Handling

```javascript
try {
    const response = await fetch(`/api/v1/glossary/${id}`);
    if (!response.ok) {
        // Status code tells us what happened
        if (response.status === 404) {
            showError('Term not found');
        } else if (response.status === 409) {
            showError('Term already exists');
        }
    }
} catch (error) {
    showError('Network error');
}
```

## 🧪 Testing Strategy

### Integration Tests

Test the **full HTTP → Database → HTTP** path:

```rust
#[actix_rt::test]
async fn test_get_glossary_not_exists() {
    let ctx = TestContext::new("test_name");
    let pool = web::Data::new(ctx.get_pool());
    let app = test::init_service(App::new().app_data(pool).service(get)).await;

    let req = test::TestRequest::get().uri("/glossary/{uuid}").to_request();
    let resp = test::call_service(&app, req).await;

    // Assert semantic status code
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

### Test Database Isolation

Each test gets an isolated database:
```rust
pub struct TestContext {
    db_name: String,
}

impl TestContext {
    pub fn new(db_name: &str) -> Self {
        // Create unique database for this test
        // Automatically dropped when TestContext is dropped
    }
}
```

### Status Code Assertions

**Always assert the correct HTTP status:**
- `200 OK` - Success
- `201 CREATED` - Resource created
- `404 NOT_FOUND` - Resource doesn't exist
- `409 CONFLICT` - Unique/FK violation
- `422 UNPROCESSABLE_ENTITY` - Validation failed
- `500 INTERNAL_SERVER_ERROR` - Server error

## 📝 Code Style Guidelines

### 1. Explicit Error Handling

```rust
// ✅ CORRECT - Let ? operator and ApiError handle it
pub async fn get(id: web::Path<String>) -> Result<impl Responder, ApiError> {
    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid UUID format"))?;

    let glossary = web::block(move || get_glossary(&mut conn, glossary_id)).await??;
    Ok(web::Json(glossary))
}

// ❌ WRONG - Manual error matching obscures intent
pub async fn get(id: web::Path<String>) -> Result<impl Responder, ErrorResp> {
    match web::block(...).await {
        Ok(Ok(g)) => Ok(web::Json(g)),
        Ok(Err(e)) => Err(ErrorResp::new(&e.to_string())),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}
```

### 2. Database Operations in web::block()

Diesel is synchronous. Always use `web::block()` for database operations:

```rust
let result = web::block(move || {
    diesel_operation(&mut conn)
}).await??;  // First ? = BlockingError, Second ? = DieselError
```

### 3. Prefer Database Constraints Over Application Logic

```rust
// ✅ CORRECT - Let database CASCADE handle it
fn delete_glossary(conn: &mut PgConnection, id: Uuid) -> Result<usize, Error> {
    diesel::delete(glossary.find(id)).execute(conn)
    // Database automatically deletes likes & history
}

// ❌ WRONG - Manual cascade is error-prone
fn delete_glossary(conn: &mut PgConnection, id: Uuid) -> Result<usize, Error> {
    diesel::delete(likes.filter(glossary_id.eq(id))).execute(conn)?;
    diesel::delete(glossary_history.filter(glossary_id.eq(id))).execute(conn)?;
    diesel::delete(glossary.find(id)).execute(conn)
    // What if one fails? Orphaned data!
}
```

## 🚀 Deployment Considerations

### Environment Variables

```bash
DATABASE_URL=postgres://user:pass@host:5432/glossary
HOST=0.0.0.0
PORT=8080
RUST_LOG=actix_web=info
```

### Health Checks

Use for Kubernetes liveness/readiness:
```yaml
livenessProbe:
  httpGet:
    path: /live
    port: 8080
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
```

### Database Migrations

Migrations run automatically on startup:
```rust
conn.run_pending_migrations(MIGRATIONS)
    .expect("failed to run migrations");
```

**Never skip migrations in production.**

### Docker Deployment

The service includes embedded migrations and static files:
```dockerfile
# Multi-stage build creates minimal image
# Migrations embedded via embed_migrations!()
# Frontend embedded via include_str!()
```

## 🔄 Development Workflow

### 1. Adding a New Endpoint

```rust
// 1. Define in v1/module.rs
#[get("/endpoint")]
pub async fn handler(pool: web::Data<DBPool>) -> Result<impl Responder, ApiError> {
    let result = web::block(move || database_operation(&mut conn)).await??;
    Ok(web::Json(result))
}

// 2. Register in main.rs
.service(v1::module::handler)

// 3. Add integration test
#[actix_rt::test]
async fn test_handler() {
    // Test with isolated database
}

// 4. Update README.md API docs
```

### 2. Database Schema Changes

```bash
# Create migration
diesel migration generate descriptive_name

# Edit up.sql and down.sql
# Always test both directions!

# Apply
diesel migration run

# Revert (test rollback)
diesel migration revert
```

### 3. Frontend Changes

Edit files in `static/`:
- `index.html` - HTML structure
- `styles.css` - Styling (use CSS variables)
- `app.js` - JavaScript logic

Changes are embedded via `include_str!()` at compile time.

## 📊 Performance Optimization

### Query Performance

1. **Use indexes** for WHERE, ORDER BY, JOIN columns
2. **Limit results** with `.limit()`
3. **Use connection pooling** (r2d2 built-in)
4. **Monitor slow queries** with `EXPLAIN ANALYZE`

### Response Compression

Actix middleware handles gzip/brotli:
```rust
.wrap(middleware::Compress::default())
```

### Static File Serving

Frontend files are embedded in binary - zero disk I/O.

## 🎓 Learning Resources

### Rust & Actix
- [Actix Web Documentation](https://actix.rs)
- [Diesel ORM Guide](https://diesel.rs)
- [Rust Book](https://doc.rust-lang.org/book/)

### PostgreSQL
- [PostgreSQL Constraints](https://www.postgresql.org/docs/current/ddl-constraints.html)
- [PostgreSQL Indexes](https://www.postgresql.org/docs/current/indexes.html)

### HTTP Semantics
- [RFC 9110 - HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)
- [MDN HTTP Status Codes](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status)

## 🔍 Common Patterns

### Pagination

```rust
#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

glossary
    .limit(limit)
    .offset((page - 1) * limit)
    .load(conn)
```

### Search

```rust
// Case-insensitive LIKE
glossary
    .filter(term.ilike(format!("%{}%", query)))
    .load(conn)
```

### Sorting

```rust
// Use indexes for DESC sorting
glossary
    .order(created_at.desc())
    .load(conn)
```

## ✅ Code Review Checklist

Before merging:
- [ ] All tests pass
- [ ] Correct HTTP status codes
- [ ] Database migrations tested (up AND down)
- [ ] No SQL injection vulnerabilities
- [ ] HTML sanitization applied
- [ ] Error messages don't leak sensitive data
- [ ] Frontend works on mobile
- [ ] Dark mode works correctly
- [ ] README updated if API changed
- [ ] No unwrap() in production code paths

---

**Remember:** Elegance is achieved not when there's nothing left to add, but when there's nothing left to take away.

This codebase embodies simplicity, correctness, and performance. Maintain these principles in all changes.

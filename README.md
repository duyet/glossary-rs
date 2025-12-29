<div align="center">
  <img src=".github/logo.png" alt="Glossary Logo" width="100" />
  <h1>Glossary</h1>
  <p>
    <strong>Production-ready glossary service with beautiful UI</strong>
  </p>
  <p>
    Open-source knowledge base built with Rust 🦀<br/>
    Powered by <a href="https://actix.rs">Actix-web</a> and <a href="https://diesel.rs">Diesel</a>
  </p>
  <p>
    <a href="https://github.com/duyet/glossary-rs/actions/workflows/build-test.yaml">
      <img src="https://github.com/duyet/glossary-rs/actions/workflows/build-test.yaml/badge.svg" />
    </a>
    <a href="https://github.com/duyet/glossary-rs/graphs/contributors" alt="Contributors">
      <img src="https://img.shields.io/github/contributors/duyet/glossary-rs" />
    </a>
    <a href="https://github.com/duyet/glossary-rs/pulse" alt="Activity">
      <img src="https://img.shields.io/github/commit-activity/m/duyet/glossary-rs" />
    </a>
  </p>
</div>

## ✨ Features

### 🎨 Modern Web Interface
- **Beautiful responsive design** with automatic dark mode
- **Real-time search** across terms and definitions
- **Alphabetically organized** glossary view
- **Popular terms** sidebar with like counts
- **Interactive like system** to highlight important definitions
- **Mobile-first** responsive design

### 🚀 Production-Ready API
- **Semantic HTTP status codes** (404, 409, 422, 500)
- **RESTful design** with proper error handling
- **Search functionality** with fuzzy matching
- **Health check endpoints** for monitoring & K8s
- **Immutable audit history** tracking all changes
- **Optimistic locking** with revision numbers

### 🛡️ Security & Performance
- **Security headers** (CSP, X-Frame-Options, X-XSS-Protection)
- **Database indexes** for optimized queries
- **ON DELETE CASCADE** for referential integrity
- **HTML sanitization** with Ammonia (XSS protection)
- **CORS support** for cross-origin requests
- **Response compression** with gzip/brotli

## 📚 API Endpoints

### Glossary Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/glossary` | List all terms grouped by first letter |
| GET | `/api/v1/glossary-popular?limit=10` | Get most liked terms |
| GET | `/api/v1/glossary-search?q=query` | 🔍 **NEW** Search terms and definitions |
| GET | `/api/v1/glossary/{id}` | Get specific term details |
| POST | `/api/v1/glossary` | Create new term |
| PUT | `/api/v1/glossary/{id}` | Update existing term |
| DELETE | `/api/v1/glossary/{id}` | Delete term |

### Likes & Engagement

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/glossary/{id}/likes` | Get all likes for a term |
| POST | `/api/v1/glossary/{id}/likes` | Like a term |
| DELETE | `/api/v1/glossary/{id}/likes` | Remove like |

### Health & Monitoring

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | 🏥 **NEW** Health check with DB status |
| GET | `/ready` | 🎯 **NEW** Kubernetes readiness probe |
| GET | `/live` | 💓 **NEW** Kubernetes liveness probe |


# Development

## Prerequisites

- Rust >= 1.26
- PostgreSQL >= 9.5

## Set up the database

Install the diesel command-line tool including the postgres feature

```bash
cargo install diesel_cli --no-default-features --features postgres
```

Check the contents of the `.env` file. 
If your database requires a password, update `DATABASE_URL` to be of the form:

```bash
DATABASE_URL=postgres://username:password@localhost/glossary
```

Tip: on MacOS, please prefer to use https://postgres.app

Then to create and set-up the database run:

```bash
diesel database setup
```

Migrate database schema:

```bash
diesel migration run
```

## Run the application

To run the application execute:

```bash
cargo run
```

Then open in your browser: http://localhost:8080

## Tests

To run the unittest, make sure to have Postgres installed in your machine.
Please export ```TEST_DATABASE_URL=postgres://localhost:5432``` to your Postgres instance.

- On the MacOS, the easiest way is install https://postgres.app
- Otherwise, you can start Postgres by Docker Compose:
   ```
   docker-compose up -d
   ```

To run the unittest:

```
cargo test
```

# Deployment

## Using Docker image

Deploy using docker image from https://github.com/duyet/glossary/pkgs/container/glossary

```bash
docker run -it \
  -e DATABASE_URL=postgres://postgres:5432/glossary \
  -p 8080:8080 \
  ghcr.io/duyet/glossary:0.1.0
```

## Using Helm chart

TBU

## Building Docker image from source

Build and deploy by using docker:

```bash
docker build -t glossary .
```

```bash
docker run -it \
  -e DATABASE_URL=postgres://postgres:5432/glossary \
  -p 8080:8080 \
  glossary 
```

# License

MIT

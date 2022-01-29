# Glossary API

Glossary services API written in Rust by [actix-web](https://actix.rs) and PostgreSQL.

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

Hint: on MacOS, please prefer to use https://postgres.app

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

```
cargo test
```

# Deployment

Build and deploy by using docker:

```bash
docker build -t glossary .
```

```bash
docker run -it -e DATABASE_URL=postgres://<database> -p 8080:8080 glossary 
```

# License

MIT

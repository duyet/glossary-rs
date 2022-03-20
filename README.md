<div align="center">
  <img src=".github/logo.png" alt="Glossary Logo" width="100" />
  <h1>Glossary API</h1>
  <p>
    Open-Source Glossary API Service written in Rust, 
    powered by https://actix.rs and https://diesel.rs.
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

# Endpoints

| Method | Endpoint | What it does |
| ------ | -------- | -------------|
| GET | ```/api/v1/glossary``` | Returns a dictionary of glossary with key is the first character of glossary terms.
| GET | ```/api/v1/glossary-popular``` | Returns an array of most popular terms by likes.
| GET |  ```/api/v1/glossary/{id}``` | Return the glossary term and defintion.
| POST | ```/api/v1/glossary``` | Create a new glossary with `term` and `defintion`.
| PUT | ```/api/v1/glossary/{id}``` | Update a glossary.
| DELETE | ```/api/v1/glossary/{id}``` | Delete a glossary.
| GET | ```/api/v1/glossary/{id}/likes``` | Return an array of likes for a glossary.
| POST | ```/api/v1/glossary/{id}/likes``` | Create a like for a glossary.
| DELETE | ```/api/v1/glossary/{id}/likes``` | Delete a like from a glossary.


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

use actix_web::{get, web, HttpResponse, Responder};
use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::DBPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub database: String,
    pub version: String,
}

/// Health check endpoint for monitoring and load balancers
/// Returns 200 OK if service is healthy, 503 if database is unreachable
#[get("/health")]
pub async fn health_check(pool: web::Data<DBPool>) -> impl Responder {
    let timestamp = Utc::now().to_rfc3339();
    let version = env!("CARGO_PKG_VERSION").to_string();

    // Try to get a database connection and execute a simple query
    let db_status = match pool.get() {
        Ok(mut conn) => {
            // Execute a simple query to verify database connectivity
            match diesel::sql_query("SELECT 1").execute(&mut conn) {
                Ok(_) => "healthy",
                Err(_) => "unhealthy",
            }
        }
        Err(_) => "unavailable",
    };

    let response = HealthResponse {
        status: if db_status == "healthy" {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        timestamp,
        database: db_status.to_string(),
        version,
    };

    if db_status == "healthy" {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

/// Readiness check - returns 200 when service is ready to accept traffic
#[get("/ready")]
pub async fn readiness_check(pool: web::Data<DBPool>) -> impl Responder {
    match pool.get() {
        Ok(mut conn) => match diesel::sql_query("SELECT 1").execute(&mut conn) {
            Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                "status": "ready",
                "timestamp": Utc::now().to_rfc3339()
            })),
            Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "not ready",
                "reason": "database query failed"
            })),
        },
        Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "not ready",
            "reason": "database connection failed"
        })),
    }
}

/// Liveness check - returns 200 as long as the service process is running
#[get("/live")]
pub async fn liveness_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "alive",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

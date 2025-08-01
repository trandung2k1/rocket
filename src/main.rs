use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::{PgPool, FromRow};
use dotenvy::dotenv;
use std::env;

#[derive(Serialize, Deserialize, FromRow)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct UpdateUser {
    name: Option<String>,
    email: Option<String>,
}

// GET /users
async fn get_users(db: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(db.get_ref())
        .await;

    match result {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(_) => HttpResponse::InternalServerError().body("Failed to fetch users"),
    }
}

// GET /users/{id}
async fn get_user(path: web::Path<Uuid>, db: web::Data<PgPool>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db.get_ref())
        .await;

    match result {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(_) => HttpResponse::InternalServerError().body("Error fetching user"),
    }
}

// POST /users
async fn create_user(db: web::Data<PgPool>, user: web::Json<CreateUser>) -> impl Responder {
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO users (id, name, email) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&user.name)
        .bind(&user.email)
        .execute(db.get_ref())
        .await;

    match result {
        Ok(_) => HttpResponse::Created().json(User {
            id,
            name: user.name.clone(),
            email: user.email.clone(),
        }),
        Err(_) => HttpResponse::InternalServerError().body("Failed to insert user"),
    }
}

// PUT /users/{id}
async fn update_user(path: web::Path<Uuid>, db: web::Data<PgPool>, user: web::Json<UpdateUser>) -> impl Responder {
    let id = path.into_inner();

    let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db.get_ref())
        .await;

    if let Ok(Some(old_user)) = existing {
        let new_name = user.name.clone().unwrap_or(old_user.name);
        let new_email = user.email.clone().unwrap_or(old_user.email);

        let result = sqlx::query("UPDATE users SET name = $1, email = $2 WHERE id = $3")
            .bind(new_name.clone())
            .bind(new_email.clone())
            .bind(id)
            .execute(db.get_ref())
            .await;

        match result {
            Ok(_) => HttpResponse::Ok().json(User {
                id,
                name: new_name,
                email: new_email,
            }),
            Err(_) => HttpResponse::InternalServerError().body("Failed to update user"),
        }
    } else {
        HttpResponse::NotFound().body("User not found")
    }
}

// DELETE /users/{id}
async fn delete_user(path: web::Path<Uuid>, db: web::Data<PgPool>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(db.get_ref())
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().body("User deleted"),
        Ok(_) => HttpResponse::NotFound().body("User not found"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to delete user"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&db_url).await.expect("Failed to connect DB");

    // Run migrations if not yet run
    sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/users", web::get().to(get_users))
            .route("/users", web::post().to(create_user))
            .route("/users/{id}", web::get().to(get_user))
            .route("/users/{id}", web::put().to(update_user))
            .route("/users/{id}", web::delete().to(delete_user))
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}

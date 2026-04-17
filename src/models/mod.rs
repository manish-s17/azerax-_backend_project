use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── User ──────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub id:             Uuid,
    pub username:       String,
    pub email:          String,
    pub password_hash:  Option<String>,
    pub bio:            String,
    pub avatar_url:     Option<String>,
    pub role:           String,
    pub is_verified:    bool,
    pub chapters_read:  i32,
    pub library_count:  i32,
    pub bookmark_count: i32,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}

/// Safe user — never exposes password_hash
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPublic {
    pub id:             Uuid,
    pub username:       String,
    pub email:          String,
    pub bio:            String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url:     Option<String>,
    pub role:           String,
    #[serde(rename = "isVerified")]
    pub is_verified:    bool,
    #[serde(rename = "chaptersRead")]
    pub chapters_read:  i32,
    #[serde(rename = "libraryCount")]
    pub library_count:  i32,
    #[serde(rename = "bookmarkCount")]
    pub bookmark_count: i32,
    #[serde(rename = "createdAt")]
    pub created_at:     DateTime<Utc>,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        Self {
            id:             u.id,
            username:       u.username,
            email:          u.email,
            bio:            u.bio,
            avatar_url:     u.avatar_url,
            role:           u.role,
            is_verified:    u.is_verified,
            chapters_read:  u.chapters_read,
            library_count:  u.library_count,
            bookmark_count: u.bookmark_count,
            created_at:     u.created_at,
        }
    }
}

// ─── Book ──────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Book {
    pub id:          Uuid,
    pub title:       String,
    pub author:      String,
    pub description: String,
    pub img_url:     String,
    pub price:       i32,
    pub stock:       i32,
    pub genre:       String,
    pub tag:         String,
    pub rating:      f64,
    pub book_type:   String,
    pub is_active:   bool,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

// ─── Manga Chapter ─────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MangaChapter {
    pub id:          Uuid,
    pub book_id:     Uuid,
    pub chapter_num: i32,
    pub title:       String,
    pub pages:       serde_json::Value,  // JSON array of filenames
    pub created_at:  DateTime<Utc>,
}

// ─── Library Entry ─────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LibraryEntry {
    pub id:           Uuid,
    pub user_id:      Uuid,
    pub book_id:      Uuid,
    pub last_chapter: i32,
    pub added_at:     DateTime<Utc>,
    // Joined fields
    pub title:        Option<String>,
    pub img_url:      Option<String>,
    pub genre:        Option<String>,
}

// ─── Reading History ───────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ReadingHistoryEntry {
    pub id:          Uuid,
    pub user_id:     Uuid,
    pub book_id:     Uuid,
    pub chapter_num: i32,
    pub page_num:    i32,
    pub read_at:     DateTime<Utc>,
    // Joined
    pub title:       Option<String>,
    pub img_url:     Option<String>,
}

// ─── Bookmark ──────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Bookmark {
    pub id:          Uuid,
    pub user_id:     Uuid,
    pub book_id:     Uuid,
    pub chapter_num: i32,
    pub page_num:    i32,
    pub created_at:  DateTime<Utc>,
    // Joined
    pub title:       Option<String>,
    pub img_url:     Option<String>,
}

// ─── Cart Item ─────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CartItem {
    pub id:       Uuid,
    pub user_id:  Uuid,
    pub book_id:  Uuid,
    pub quantity: i32,
    pub added_at: DateTime<Utc>,
    // Joined
    pub title:    Option<String>,
    pub price:    Option<i32>,
    pub img_url:  Option<String>,
}

// ─── Order ─────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id:          Uuid,
    pub user_id:     Uuid,
    pub total_price: i32,
    pub status:      String,
    pub created_at:  DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct OrderItem {
    pub id:         Uuid,
    pub order_id:   Uuid,
    pub book_id:    Uuid,
    pub quantity:   i32,
    pub unit_price: i32,
    // Joined
    pub title:      Option<String>,
    pub img_url:    Option<String>,
}

// ─── News ──────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NewsArticle {
    pub id:         Uuid,
    pub title:      String,
    pub summary:    String,
    pub category:   String,
    pub tag:        String,
    pub img_url:    String,
    pub author:     String,
    pub is_hot:     bool,
    pub created_at: DateTime<Utc>,
}

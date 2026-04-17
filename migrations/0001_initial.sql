-- ============================================================
-- AEZARX Database Schema
-- Run: sqlx migrate run
-- ============================================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ── Users ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username      VARCHAR(50)  NOT NULL UNIQUE,
    email         VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255),                    -- NULL for magic-link-only users
    bio           TEXT         NOT NULL DEFAULT '',
    avatar_url    VARCHAR(500),
    role          VARCHAR(20)  NOT NULL DEFAULT 'user',  -- 'user' | 'admin'
    is_verified   BOOLEAN      NOT NULL DEFAULT FALSE,
    chapters_read INTEGER      NOT NULL DEFAULT 0,
    library_count INTEGER      NOT NULL DEFAULT 0,
    bookmark_count INTEGER     NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ── Email verification tokens ─────────────────────────────────
CREATE TABLE IF NOT EXISTS email_verifications (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Password reset tokens ─────────────────────────────────────
CREATE TABLE IF NOT EXISTS password_resets (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Magic link tokens ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS magic_links (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Books (manga + self-help books) ──────────────────────────
CREATE TABLE IF NOT EXISTS books (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title       VARCHAR(255) NOT NULL,
    author      VARCHAR(255) NOT NULL DEFAULT 'Unknown',
    description TEXT         NOT NULL DEFAULT '',
    img_url     VARCHAR(500) NOT NULL DEFAULT '',
    price       INTEGER      NOT NULL DEFAULT 0,   -- stored in paise/cents
    stock       INTEGER      NOT NULL DEFAULT 0,
    genre       VARCHAR(100) NOT NULL DEFAULT 'manga',
    tag         VARCHAR(100) NOT NULL DEFAULT '',
    rating      NUMERIC(3,1) NOT NULL DEFAULT 0.0,
    book_type   VARCHAR(20)  NOT NULL DEFAULT 'manga',  -- 'manga' | 'book'
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ── Manga chapters index ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS manga_chapters (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    book_id     UUID    NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_num INTEGER NOT NULL,
    title       VARCHAR(255) NOT NULL DEFAULT '',
    pages       JSONB   NOT NULL DEFAULT '[]',  -- array of page filenames
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(book_id, chapter_num)
);

-- ── User library ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_library (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id      UUID    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id      UUID    NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    last_chapter INTEGER NOT NULL DEFAULT 0,
    added_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, book_id)
);

-- ── Reading history ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS reading_history (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id     UUID    NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_num INTEGER NOT NULL DEFAULT 1,
    page_num    INTEGER NOT NULL DEFAULT 1,
    read_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, book_id, chapter_num)
);

-- ── Bookmarks ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bookmarks (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id     UUID    NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_num INTEGER NOT NULL DEFAULT 1,
    page_num    INTEGER NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Cart items ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS cart_items (
    id         UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id    UUID    NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    quantity   INTEGER NOT NULL DEFAULT 1,
    added_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, book_id)
);

-- ── Orders ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS orders (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    total_price INTEGER     NOT NULL DEFAULT 0,
    status      VARCHAR(30) NOT NULL DEFAULT 'confirmed',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Order items ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS order_items (
    id         UUID    PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id   UUID    NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    book_id    UUID    NOT NULL REFERENCES books(id),
    quantity   INTEGER NOT NULL DEFAULT 1,
    unit_price INTEGER NOT NULL DEFAULT 0
);

-- ── News articles ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS news (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title      VARCHAR(500) NOT NULL,
    summary    TEXT         NOT NULL DEFAULT '',
    category   VARCHAR(50)  NOT NULL DEFAULT 'manga',   -- manga|anime|books
    tag        VARCHAR(100) NOT NULL DEFAULT '',
    img_url    VARCHAR(500) NOT NULL DEFAULT '',
    author     VARCHAR(100) NOT NULL DEFAULT 'AEZARX',
    is_hot     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ── Indexes ───────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_users_email    ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_books_genre    ON books(genre);
CREATE INDEX IF NOT EXISTS idx_books_type     ON books(book_type);
CREATE INDEX IF NOT EXISTS idx_books_title    ON books USING gin(to_tsvector('english', title));
CREATE INDEX IF NOT EXISTS idx_orders_user    ON orders(user_id);
CREATE INDEX IF NOT EXISTS idx_history_user   ON reading_history(user_id);
CREATE INDEX IF NOT EXISTS idx_library_user   ON user_library(user_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_user ON bookmarks(user_id);
CREATE INDEX IF NOT EXISTS idx_ev_token       ON email_verifications(token);
CREATE INDEX IF NOT EXISTS idx_pr_token       ON password_resets(token);
CREATE INDEX IF NOT EXISTS idx_ml_token       ON magic_links(token);

-- ── Seed: Sample books ────────────────────────────────────────
INSERT INTO books (title, author, description, img_url, price, stock, genre, tag, rating, book_type) VALUES
  ('Dragon Ball',      'Akira Toriyama',   'Follow Goku from a young Saiyan warrior to the universe''s greatest fighter.',                                    '/images/DB.jpg',           299, 100, 'shonen',      'Classic',     4.8, 'manga'),
  ('Naruto',           'Masashi Kishimoto','A tale of a young ninja outcast who dreams of becoming Hokage.',                                                   '/images/Naruto.jpg',       399, 100, 'shonen',      'Legend',      4.6, 'manga'),
  ('One Piece',        'Eiichiro Oda',     'Monkey D. Luffy sets sail to become King of the Pirates.',                                                        '/images/OnePiece.jpg',     399, 100, 'shonen',      '🔥 Hot',      4.9, 'manga'),
  ('Bleach',           'Tite Kubo',        'Ichigo Kurosaki becomes a Soul Reaper to protect his family.',                                                    '/images/Bleach.jpg',       249, 100, 'shonen',      'Popular',     4.7, 'manga'),
  ('Demon Slayer',     'Koyoharu Gotouge', 'Tanjiro Kamado''s journey to avenge his family and cure his demon sister.',                                       '/images/DS.jpg',           399, 100, 'shonen',      'Trending',    4.8, 'manga'),
  ('Jujutsu Kaisen',   'Gege Akutami',     'Yuji Itadori swallows a cursed finger and becomes host to the most powerful curse in history.',                   '/images/JJK.jpg',          199, 100, 'shonen',      'New',         4.5, 'manga'),
  ('Solo Levelling',   'Chugong',          'The weakest hunter awakens a unique power to level up infinitely.',                                               '/images/Sololevelling.jpg',299, 100, 'manhwa',      'Manhwa',      4.6, 'manga'),
  ('Vinland Saga',     'Makoto Yukimura',  'Viking warrior Thorfinn seeks revenge, then redemption.',                                                         '/images/VS.jpg',           349, 100, 'seinen',      'Masterpiece', 4.9, 'manga'),
  ('Berserk',          'Kentaro Miura',    'Guts, a lone swordsman, carries a mountain of trauma and a sword bigger than most men.',                          '/images/Berserk.jpg',      249, 100, 'seinen',      'Dark',        4.6, 'manga'),
  ('Blue Lock',        'Muneyuki Kaneshiro','300 strikers compete in a radical program to forge Japan''s ultimate ego-driven forward.',                        '/images/Blue Lock.jpg',    199, 100, 'shonen',      'Sports',      4.7, 'manga'),
  ('Attack on Titan',  'Hajime Isayama',   'Humanity''s last survivors battle man-eating giants behind massive walls.',                                        '/images/AOT.jpg',          299, 100, 'seinen',      'Epic',        4.8, 'manga'),
  ('Chainsaw Man',     'Tatsuki Fujimoto', 'Denji merges with his pet devil and becomes Chainsaw Man.',                                                       '/images/Chainsawman.jpg',  399, 100, 'seinen',      'Weird',       4.5, 'manga'),
  ('Death Note',       'Tsugumi Ohba',     'Light Yagami finds a notebook that kills anyone whose name is written in it.',                                    '/images/DN.jpg',           249, 100, 'seinen',      'Thriller',    4.4, 'manga'),
  ('Atomic Habits',    'James Clear',      'James Clear''s definitive guide to building good habits.',                                                        '/images/ATOMIC-HABITS.jpg',349, 100, 'habit',       'Bestseller',  4.9, 'book'),
  ('Deep Work',        'Cal Newport',      'The ability to focus without distraction is the superpower of the 21st century.',                                 '/images/DeepWork.jpg',     299, 100, 'productivity','Focus',       4.7, 'book'),
  ('The Subtle Art',   'Mark Manson',      'Mark Manson''s counterintuitive approach to living a good life.',                                                 '/images/SubtleArt.jpg',    249, 100, 'mindset',     'Mindset',     4.6, 'book'),
  ('48 Laws of Power', 'Robert Greene',    'Robert Greene''s masterwork on the nature of power.',                                                             '/images/Power.jpg',        399, 100, 'psychology',  'Strategy',    4.8, 'book'),
  ('Rich Dad Poor Dad','Robert Kiyosaki',  'Kiyosaki''s classic contrast between a rich mindset and a poor mindset.',                                         '/images/Rich.jpg',         249, 100, 'finance',     'Finance',     4.6, 'book')
ON CONFLICT DO NOTHING;

-- Seed: Sample news
INSERT INTO news (title, summary, category, tag, img_url, author, is_hot) VALUES
  ('One Piece Chapter 1120: The Final War Begins', 'Eiichiro Oda''s mega-series reaches its most explosive chapter yet.', 'manga', 'Chapter Release', '/images/OnePiece.jpg', 'MangaDesk', TRUE),
  ('Berserk: New Studio Confirmed for 2026 Adaptation', 'After years of waiting, Berserk fans finally get a proper TV adaptation.', 'anime', 'Anime Announcement', '/images/Berserk.jpg', 'AniNews', TRUE),
  ('Solo Levelling Breaks 10 Million Copies in India', 'The manhwa phenomenon continues to shatter records globally.', 'manga', 'Sales Record', '/images/Sololevelling.jpg', 'BookBeat', FALSE),
  ('James Clear on Atomic Habits 2.0 Coming 2027', 'Clear hints at a follow-up to his 20-million-copy bestseller.', 'books', 'Author Interview', '/images/ATOMIC-HABITS.jpg', 'ReadDaily', FALSE)
ON CONFLICT DO NOTHING;
